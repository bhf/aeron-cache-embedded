package com.bhf.aeroncache.client.bidi;

import com.bhf.aeroncache.models.CacheItem;
import com.bhf.aeroncache.models.CacheUpdateEvent;
import com.bhf.aeroncache.models.CancelItemRemovalResponse;
import com.bhf.aeroncache.models.ClearCacheResponse;
import com.bhf.aeroncache.models.CounterItem;
import com.bhf.aeroncache.models.CounterResponse;
import com.bhf.aeroncache.models.CounterUpdateEvent;
import com.bhf.aeroncache.models.CreateResponse;
import com.bhf.aeroncache.models.DeleteCacheResponse;
import com.bhf.aeroncache.models.DeleteItemResponse;
import com.bhf.aeroncache.models.GetCacheResponse;
import com.bhf.aeroncache.models.GetCountersResponse;
import com.bhf.aeroncache.models.GetItemResponse;
import com.bhf.aeroncache.models.PatchItemResponse;
import com.bhf.aeroncache.models.PutItemResponse;
import com.bhf.aeroncache.models.StatEntry;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.WebSocket;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionStage;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.ThreadFactory;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.function.Consumer;

/**
 * High-level client for the Aeron Cache bidirectional WebSocket transport ({@code /api/ws/v1/bidi}) —
 * the JSON/WebSocket analogue of {@link com.bhf.aeroncache.client.gateway.AeronGatewayClient} and an
 * alternative to the HTTP+WS {@link com.bhf.aeroncache.client.AeronCacheClient}.
 * <p>
 * A single persistent WebSocket connection carries the full cache + counter command surface plus dynamic
 * subscribe/unsubscribe, multiplexed by a client-minted {@code correlationId} that the server echoes on
 * every correlated frame. Each command mints a correlation id, registers a pending
 * {@link CompletableFuture}, and completes it when the correlated {@code commandResponse} (or
 * {@code error}) frame arrives. Synchronous variants simply block on the future.
 * <p>
 * <b>Stream-update routing.</b> The server stamps {@code streamUpdate.correlationId} with the id of the
 * <em>command that caused the update</em> (the update's requestId), not the subscription's correlation
 * id. Stream updates are therefore routed to listeners by {@code cacheId}, and the causing command's id
 * is surfaced as {@link CacheUpdateEvent#getRequestId()} / {@link CounterUpdateEvent#getRequestId()}.
 */
public class AeronBidiClient implements AutoCloseable {

    private static final System.Logger LOG = System.getLogger(AeronBidiClient.class.getName());

    private static final long DEFAULT_REQUEST_TIMEOUT_MS = 10_000L;

    // Client -> server op discriminators (WsOp).
    private static final String CREATE_CACHE = "CREATE_CACHE";
    private static final String ADD_CACHE_ENTRY = "ADD_CACHE_ENTRY";
    private static final String PATCH_CACHE_ENTRY = "PATCH_CACHE_ENTRY";
    private static final String GET_CACHE_ENTRY = "GET_CACHE_ENTRY";
    private static final String CLEAR_CACHE = "CLEAR_CACHE";
    private static final String DELETE_CACHE = "DELETE_CACHE";
    private static final String REMOVE_CACHE_ENTRY = "REMOVE_CACHE_ENTRY";
    private static final String CANCEL_CACHE_ITEM_REMOVAL = "CANCEL_CACHE_ITEM_REMOVAL";
    private static final String GET_CACHE_ENTRIES = "GET_CACHE_ENTRIES";
    private static final String GET_CACHE_STATS = "GET_CACHE_STATS";
    private static final String CREATE_COUNTER_CACHE = "CREATE_COUNTER_CACHE";
    private static final String ADD_COUNTER_ENTRY = "ADD_COUNTER_ENTRY";
    private static final String GET_COUNTER_ENTRY = "GET_COUNTER_ENTRY";
    private static final String CLEAR_COUNTER_CACHE = "CLEAR_COUNTER_CACHE";
    private static final String DELETE_COUNTER_CACHE = "DELETE_COUNTER_CACHE";
    private static final String REMOVE_COUNTER_ENTRY = "REMOVE_COUNTER_ENTRY";
    private static final String CANCEL_COUNTER_ITEM_REMOVAL = "CANCEL_COUNTER_ITEM_REMOVAL";
    private static final String GET_COUNTER_ENTRIES = "GET_COUNTER_ENTRIES";
    private static final String GET_COUNTER_STATS = "GET_COUNTER_STATS";
    private static final String INCREMENT_COUNTER_ENTRY = "INCREMENT_COUNTER_ENTRY";
    private static final String DECREMENT_COUNTER_ENTRY = "DECREMENT_COUNTER_ENTRY";
    private static final String SET_COUNTER_ENTRY = "SET_COUNTER_ENTRY";

    private final String wsUrl;
    private final long requestTimeoutMs;
    private final HttpClient httpClient;
    private final ObjectMapper objectMapper;
    private final ExecutorService sendExecutor =
            Executors.newSingleThreadExecutor(daemonFactory("aeron-bidi-send"));

    private final Object connectLock = new Object();
    private volatile WebSocket webSocket;

    // correlationId -> pending single-response command (resolves to the commandResponse frame).
    private final Map<String, CompletableFuture<JsonNode>> pendingCommands = new ConcurrentHashMap<>();
    // correlationId -> accumulating entries request.
    private final Map<String, EntriesAccumulator> pendingEntries = new ConcurrentHashMap<>();
    // correlationId -> accumulating stats request.
    private final Map<String, StatsAccumulator> pendingStats = new ConcurrentHashMap<>();
    // subscribe correlationId -> pending subscribe-ack barrier (resolves to the acked cacheIds).
    private final Map<String, CompletableFuture<List<String>>> pendingSubscribeAcks = new ConcurrentHashMap<>();
    // cacheId -> subscription registrations. Stream updates route by cacheId (see class javadoc).
    private final Map<String, List<SubRegistration>> subscriptions = new ConcurrentHashMap<>();

    // ------------------------------------------------------------------ construction

    public AeronBidiClient(String wsUrl) {
        this(wsUrl, DEFAULT_REQUEST_TIMEOUT_MS);
    }

    public AeronBidiClient(String wsUrl, long requestTimeoutMs) {
        this.wsUrl = wsUrl.endsWith("/") ? wsUrl.substring(0, wsUrl.length() - 1) : wsUrl;
        this.requestTimeoutMs = requestTimeoutMs;
        this.httpClient = HttpClient.newHttpClient();
        this.objectMapper = new ObjectMapper()
                .configure(com.fasterxml.jackson.databind.DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);
    }

    // ------------------------------------------------------------------ lifecycle

    /** Open the WebSocket connection eagerly. Connection is otherwise established lazily on first send. */
    public void connect() {
        ensureConnected();
    }

    private WebSocket ensureConnected() {
        final WebSocket existing = webSocket;
        if (existing != null) {
            return existing;
        }
        synchronized (connectLock) {
            if (webSocket != null) {
                return webSocket;
            }
            try {
                webSocket = httpClient.newWebSocketBuilder()
                        .buildAsync(URI.create(wsUrl + "/api/ws/v1/bidi"), new BidiListener())
                        .get(requestTimeoutMs, TimeUnit.MILLISECONDS);
                return webSocket;
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                throw new RuntimeException("Interrupted connecting to " + wsUrl, e);
            } catch (ExecutionException | TimeoutException e) {
                throw new RuntimeException("Failed to connect to " + wsUrl + "/api/ws/v1/bidi", e);
            }
        }
    }

    @Override
    public void close() {
        final WebSocket ws = webSocket;
        webSocket = null;
        if (ws != null) {
            try {
                ws.sendClose(WebSocket.NORMAL_CLOSURE, "client closed");
            } catch (Exception ignored) {
                // best-effort
            }
        }
        sendExecutor.shutdownNow();
        failAllPending(new IllegalStateException("AeronBidiClient closed"));
    }

    // ------------------------------------------------------------------ cache commands

    public CompletableFuture<CreateResponse> createCacheAsync(String cacheId) {
        return command(CREATE_CACHE, cacheId, null, null, 0L, 0L)
                .thenApply(r -> createResponse(text(r, "cacheId"), text(r, "status")));
    }

    public CreateResponse createCache(String cacheId) throws Exception {
        return await(createCacheAsync(cacheId));
    }

    public CompletableFuture<PutItemResponse> putItemAsync(String cacheId, String key, String value) {
        return putTimedItemAsync(cacheId, key, value, 0L);
    }

    public PutItemResponse putItem(String cacheId, String key, String value) throws Exception {
        return await(putItemAsync(cacheId, key, value));
    }

    public CompletableFuture<PutItemResponse> putTimedItemAsync(String cacheId, String key, String value, long ttl) {
        return command(ADD_CACHE_ENTRY, cacheId, key, value, ttl, 0L)
                .thenApply(r -> putItemResponse(text(r, "cacheId"), text(r, "key"), text(r, "status")));
    }

    public PutItemResponse putTimedItem(String cacheId, String key, String value, long ttl) throws Exception {
        return await(putTimedItemAsync(cacheId, key, value, ttl));
    }

    public CompletableFuture<PatchItemResponse> patchItemAsync(String cacheId, String key, String value) {
        return command(PATCH_CACHE_ENTRY, cacheId, key, value, 0L, 0L)
                .thenApply(r -> patchItemResponse(text(r, "cacheId"), text(r, "key"), text(r, "status")));
    }

    public PatchItemResponse patchItem(String cacheId, String key, String value) throws Exception {
        return await(patchItemAsync(cacheId, key, value));
    }

    public CompletableFuture<GetItemResponse> getItemAsync(String cacheId, String key) {
        return command(GET_CACHE_ENTRY, cacheId, key, null, 0L, 0L)
                .thenApply(r -> getItemResponse(text(r, "cacheId"), text(r, "key"), text(r, "value"), text(r, "status")));
    }

    public GetItemResponse getItem(String cacheId, String key) throws Exception {
        return await(getItemAsync(cacheId, key));
    }

    public CompletableFuture<DeleteItemResponse> deleteItemAsync(String cacheId, String key) {
        return command(REMOVE_CACHE_ENTRY, cacheId, key, null, 0L, 0L)
                .thenApply(r -> deleteItemResponse(text(r, "cacheId"), text(r, "key"), text(r, "status")));
    }

    public DeleteItemResponse deleteItem(String cacheId, String key) throws Exception {
        return await(deleteItemAsync(cacheId, key));
    }

    public CompletableFuture<CancelItemRemovalResponse> cancelItemRemovalAsync(String cacheId, String key) {
        return command(CANCEL_CACHE_ITEM_REMOVAL, cacheId, key, null, 0L, 0L)
                .thenApply(r -> cancelItemRemovalResponse(text(r, "cacheId"), text(r, "key"), text(r, "status")));
    }

    public CancelItemRemovalResponse cancelItemRemoval(String cacheId, String key) throws Exception {
        return await(cancelItemRemovalAsync(cacheId, key));
    }

    public CompletableFuture<ClearCacheResponse> clearCacheAsync(String cacheId) {
        return command(CLEAR_CACHE, cacheId, null, null, 0L, 0L)
                .thenApply(r -> clearCacheResponse(text(r, "cacheId"), text(r, "status")));
    }

    public ClearCacheResponse clearCache(String cacheId) throws Exception {
        return await(clearCacheAsync(cacheId));
    }

    public CompletableFuture<DeleteCacheResponse> deleteCacheAsync(String cacheId) {
        return command(DELETE_CACHE, cacheId, null, null, 0L, 0L)
                .thenApply(r -> deleteCacheResponse(text(r, "cacheId"), text(r, "status")));
    }

    public DeleteCacheResponse deleteCache(String cacheId) throws Exception {
        return await(deleteCacheAsync(cacheId));
    }

    public CompletableFuture<GetCacheResponse> getCacheItemsAsync(String cacheId) {
        return entries(GET_CACHE_ENTRIES, cacheId).thenApply(acc -> {
            final List<CacheItem> items = new ArrayList<>();
            acc.items.forEach((k, v) -> items.add(new CacheItem(k, v == null || v.isNull() ? null : v.asText())));
            final GetCacheResponse response = new GetCacheResponse();
            response.setCacheId(acc.cacheId);
            response.setOperationStatus(acc.status);
            response.setItems(items);
            return response;
        });
    }

    public GetCacheResponse getCacheItems(String cacheId) throws Exception {
        return await(getCacheItemsAsync(cacheId));
    }

    public CompletableFuture<List<StatEntry>> getStatsAsync() {
        return stats(GET_CACHE_STATS).thenApply(acc -> acc.stats);
    }

    public List<StatEntry> getStats() throws Exception {
        return await(getStatsAsync());
    }

    // ------------------------------------------------------------------ counter commands

    public CompletableFuture<CreateResponse> createCounterCacheAsync(String cacheId) {
        return command(CREATE_COUNTER_CACHE, cacheId, null, null, 0L, 0L)
                .thenApply(r -> createResponse(text(r, "cacheId"), text(r, "status")));
    }

    public CreateResponse createCounterCache(String cacheId) throws Exception {
        return await(createCounterCacheAsync(cacheId));
    }

    public CompletableFuture<PutItemResponse> putCounterAsync(String cacheId, String key, long value) {
        return putTimedCounterAsync(cacheId, key, value, 0L);
    }

    public PutItemResponse putCounter(String cacheId, String key, long value) throws Exception {
        return await(putCounterAsync(cacheId, key, value));
    }

    public CompletableFuture<PutItemResponse> putTimedCounterAsync(String cacheId, String key, long value, long ttl) {
        return command(ADD_COUNTER_ENTRY, cacheId, key, null, ttl, value)
                .thenApply(r -> putItemResponse(text(r, "cacheId"), text(r, "key"), text(r, "status")));
    }

    public PutItemResponse putTimedCounter(String cacheId, String key, long value, long ttl) throws Exception {
        return await(putTimedCounterAsync(cacheId, key, value, ttl));
    }

    public CompletableFuture<CounterResponse> getCounterAsync(String cacheId, String key) {
        return command(GET_COUNTER_ENTRY, cacheId, key, null, 0L, 0L).thenApply(this::counterResponse);
    }

    public CounterResponse getCounter(String cacheId, String key) throws Exception {
        return await(getCounterAsync(cacheId, key));
    }

    public CompletableFuture<DeleteItemResponse> deleteCounterAsync(String cacheId, String key) {
        return command(REMOVE_COUNTER_ENTRY, cacheId, key, null, 0L, 0L)
                .thenApply(r -> deleteItemResponse(text(r, "cacheId"), text(r, "key"), text(r, "status")));
    }

    public DeleteItemResponse deleteCounter(String cacheId, String key) throws Exception {
        return await(deleteCounterAsync(cacheId, key));
    }

    public CompletableFuture<CancelItemRemovalResponse> cancelCounterItemRemovalAsync(String cacheId, String key) {
        return command(CANCEL_COUNTER_ITEM_REMOVAL, cacheId, key, null, 0L, 0L)
                .thenApply(r -> cancelItemRemovalResponse(text(r, "cacheId"), text(r, "key"), text(r, "status")));
    }

    public CancelItemRemovalResponse cancelCounterItemRemoval(String cacheId, String key) throws Exception {
        return await(cancelCounterItemRemovalAsync(cacheId, key));
    }

    public CompletableFuture<ClearCacheResponse> clearCounterCacheAsync(String cacheId) {
        return command(CLEAR_COUNTER_CACHE, cacheId, null, null, 0L, 0L)
                .thenApply(r -> clearCacheResponse(text(r, "cacheId"), text(r, "status")));
    }

    public ClearCacheResponse clearCounterCache(String cacheId) throws Exception {
        return await(clearCounterCacheAsync(cacheId));
    }

    public CompletableFuture<DeleteCacheResponse> deleteCounterCacheAsync(String cacheId) {
        return command(DELETE_COUNTER_CACHE, cacheId, null, null, 0L, 0L)
                .thenApply(r -> deleteCacheResponse(text(r, "cacheId"), text(r, "status")));
    }

    public DeleteCacheResponse deleteCounterCache(String cacheId) throws Exception {
        return await(deleteCounterCacheAsync(cacheId));
    }

    public CompletableFuture<CounterResponse> incrementCounterAsync(String cacheId, String key, long amount) {
        return command(INCREMENT_COUNTER_ENTRY, cacheId, key, null, 0L, amount).thenApply(this::counterResponse);
    }

    public CounterResponse incrementCounter(String cacheId, String key, long amount) throws Exception {
        return await(incrementCounterAsync(cacheId, key, amount));
    }

    public CompletableFuture<CounterResponse> decrementCounterAsync(String cacheId, String key, long amount) {
        return command(DECREMENT_COUNTER_ENTRY, cacheId, key, null, 0L, amount).thenApply(this::counterResponse);
    }

    public CounterResponse decrementCounter(String cacheId, String key, long amount) throws Exception {
        return await(decrementCounterAsync(cacheId, key, amount));
    }

    public CompletableFuture<CounterResponse> setCounterAsync(String cacheId, String key, long value) {
        return command(SET_COUNTER_ENTRY, cacheId, key, null, 0L, value).thenApply(this::counterResponse);
    }

    public CounterResponse setCounter(String cacheId, String key, long value) throws Exception {
        return await(setCounterAsync(cacheId, key, value));
    }

    public CompletableFuture<GetCountersResponse> getCounterItemsAsync(String cacheId) {
        return entries(GET_COUNTER_ENTRIES, cacheId).thenApply(acc -> {
            final List<CounterItem> items = new ArrayList<>();
            acc.items.forEach((k, v) -> items.add(new CounterItem(k, v == null || v.isNull() ? 0L : v.asLong())));
            final GetCountersResponse response = new GetCountersResponse();
            response.setCacheId(acc.cacheId);
            response.setOperationStatus(acc.status);
            response.setItems(items);
            return response;
        });
    }

    public GetCountersResponse getCounterItems(String cacheId) throws Exception {
        return await(getCounterItemsAsync(cacheId));
    }

    public CompletableFuture<List<StatEntry>> getCounterStatsAsync() {
        return stats(GET_COUNTER_STATS).thenApply(acc -> acc.stats);
    }

    public List<StatEntry> getCounterStats() throws Exception {
        return await(getCounterStatsAsync());
    }

    // ------------------------------------------------------------------ subscriptions

    public BidiSubscription subscribe(String cacheId, Consumer<CacheUpdateEvent> listener) {
        return subscribe(cacheId, false, null, null, listener);
    }

    public BidiSubscription subscribe(String cacheId, boolean sendSnapshot, Consumer<CacheUpdateEvent> listener) {
        return subscribe(cacheId, sendSnapshot, null, null, listener);
    }

    /**
     * Subscribe to streaming updates for a regular cache, with an optional {@code key} filter and
     * subscription {@code mode} ({@code "FULL"} full values or {@code "PATCH"} deltas). {@code PATCH_ITEM}
     * updates arrive through the same {@link CacheUpdateEvent} channel.
     *
     * @return an {@link AutoCloseable} handle; closing it unsubscribes.
     */
    public BidiSubscription subscribe(String cacheId, boolean sendSnapshot, String key, String mode,
                                      Consumer<CacheUpdateEvent> listener) {
        return doSubscribe(cacheId, sendSnapshot, key, mode, false,
                new SubRegistration(listener, null, false));
    }

    public BidiSubscription subscribeCounter(String cacheId, Consumer<CounterUpdateEvent> listener) {
        return subscribeCounter(cacheId, false, null, listener);
    }

    public BidiSubscription subscribeCounter(String cacheId, boolean sendSnapshot, Consumer<CounterUpdateEvent> listener) {
        return subscribeCounter(cacheId, sendSnapshot, null, listener);
    }

    /**
     * Subscribe to streaming updates for a counter cache, with an optional {@code key} filter. Counter
     * stream values are JSON numbers surfaced as {@link CounterUpdateEvent#getItemValue()} (int64).
     *
     * @return an {@link AutoCloseable} handle; closing it unsubscribes.
     */
    public BidiSubscription subscribeCounter(String cacheId, boolean sendSnapshot, String key,
                                             Consumer<CounterUpdateEvent> listener) {
        return doSubscribe(cacheId, sendSnapshot, key, null, true,
                new SubRegistration(null, listener, true));
    }

    private BidiSubscription doSubscribe(String cacheId, boolean sendSnapshot, String key, String mode,
                                         boolean counters, SubRegistration registration) {
        final String cid = UUID.randomUUID().toString();
        registration.correlationId = cid;
        subscriptions.computeIfAbsent(cacheId, k -> new CopyOnWriteArrayList<>()).add(registration);

        final CompletableFuture<List<String>> ack = new CompletableFuture<>();
        pendingSubscribeAcks.put(cid, ack);

        final ObjectNode frame = objectMapper.createObjectNode();
        frame.put("type", "subscribe");
        frame.put("correlationId", cid);
        frame.put("counters", counters);
        frame.put("sendSnapshot", sendSnapshot);
        final ObjectNode selector = objectMapper.createObjectNode();
        selector.put("cacheId", cacheId);
        if (key != null) {
            selector.put("key", key);
        }
        if (mode != null) {
            selector.put("mode", mode.toUpperCase());
        }
        frame.putArray("caches").add(selector);
        send(frame);

        awaitSubscribeAck(cid, ack);
        return new BidiSubscription(this, cid, cacheId, counters);
    }

    void unsubscribe(String correlationId, String cacheId, boolean counters) {
        final List<SubRegistration> regs = subscriptions.get(cacheId);
        if (regs != null) {
            regs.removeIf(r -> correlationId.equals(r.correlationId));
            if (regs.isEmpty()) {
                subscriptions.remove(cacheId, regs);
            }
        }
        final ObjectNode frame = objectMapper.createObjectNode();
        frame.put("type", "unsubscribe");
        frame.put("correlationId", UUID.randomUUID().toString());
        frame.put("counters", counters);
        frame.put("cacheId", cacheId);
        send(frame);
    }

    private void awaitSubscribeAck(String correlationId, CompletableFuture<List<String>> ack) {
        try {
            ack.get(requestTimeoutMs, TimeUnit.MILLISECONDS);
        } catch (TimeoutException e) {
            LOG.log(System.Logger.Level.WARNING,
                    "No subscribe ack for {0} within {1}ms; proceeding", correlationId, requestTimeoutMs);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
        } catch (ExecutionException e) {
            // Barrier failed (e.g. client closed); proceed regardless.
        } finally {
            pendingSubscribeAcks.remove(correlationId);
        }
    }

    // ------------------------------------------------------------------ command plumbing

    private CompletableFuture<JsonNode> command(String op, String cacheId, String key, String value,
                                                long ttl, long counterValue) {
        final String cid = UUID.randomUUID().toString();
        final CompletableFuture<JsonNode> future = new CompletableFuture<>();
        pendingCommands.put(cid, future);
        future.whenComplete((r, e) -> pendingCommands.remove(cid));

        final ObjectNode frame = objectMapper.createObjectNode();
        frame.put("type", "command");
        frame.put("correlationId", cid);
        frame.put("op", op);
        frame.put("cacheId", cacheId);
        frame.put("key", key);
        frame.put("value", value);
        frame.put("ttl", ttl);
        frame.put("counterValue", counterValue);
        send(frame);

        future.orTimeout(requestTimeoutMs, TimeUnit.MILLISECONDS);
        return future;
    }

    private CompletableFuture<EntriesAccumulator> entries(String op, String cacheId) {
        final String cid = UUID.randomUUID().toString();
        final EntriesAccumulator acc = new EntriesAccumulator(cacheId);
        pendingEntries.put(cid, acc);
        acc.future.whenComplete((r, e) -> pendingEntries.remove(cid));
        sendBatchCommand(cid, op, cacheId);
        acc.future.orTimeout(requestTimeoutMs, TimeUnit.MILLISECONDS);
        return acc.future;
    }

    private CompletableFuture<StatsAccumulator> stats(String op) {
        final String cid = UUID.randomUUID().toString();
        final StatsAccumulator acc = new StatsAccumulator();
        pendingStats.put(cid, acc);
        acc.future.whenComplete((r, e) -> pendingStats.remove(cid));
        sendBatchCommand(cid, op, null);
        acc.future.orTimeout(requestTimeoutMs, TimeUnit.MILLISECONDS);
        return acc.future;
    }

    private void sendBatchCommand(String cid, String op, String cacheId) {
        final ObjectNode frame = objectMapper.createObjectNode();
        frame.put("type", "command");
        frame.put("correlationId", cid);
        frame.put("op", op);
        frame.put("cacheId", cacheId);
        frame.put("key", (String) null);
        frame.put("value", (String) null);
        frame.put("ttl", 0L);
        frame.put("counterValue", 0L);
        send(frame);
    }

    private void send(ObjectNode frame) {
        ensureConnected();
        final String text = frame.toString();
        sendExecutor.execute(() -> {
            try {
                final WebSocket ws = webSocket;
                if (ws != null) {
                    ws.sendText(text, true).get();
                }
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            } catch (Exception e) {
                LOG.log(System.Logger.Level.WARNING, "Failed to send frame", e);
            }
        });
    }

    private <T> T await(CompletableFuture<T> future) throws Exception {
        try {
            return future.get(requestTimeoutMs * 2, TimeUnit.MILLISECONDS);
        } catch (ExecutionException e) {
            final Throwable cause = e.getCause();
            if (cause instanceof Exception ex) {
                throw ex;
            }
            throw e;
        }
    }

    // ------------------------------------------------------------------ dispatch

    private void dispatch(JsonNode msg) {
        final String type = text(msg, "type");
        final String cid = text(msg, "correlationId");
        if (type == null) {
            return;
        }
        switch (type) {
            case "commandResponse" -> {
                final CompletableFuture<JsonNode> fut = pendingCommands.remove(cid);
                if (fut != null) {
                    fut.complete(msg);
                }
            }
            case "entries" -> {
                final EntriesAccumulator acc = pendingEntries.get(cid);
                if (acc != null) {
                    final JsonNode items = msg.get("items");
                    if (items != null && items.isObject()) {
                        items.fields().forEachRemaining(e -> acc.items.put(e.getKey(), e.getValue()));
                    }
                    acc.cacheId = text(msg, "cacheId");
                    acc.status = text(msg, "status");
                    if (msg.path("endOfBatch").asBoolean(false)) {
                        pendingEntries.remove(cid);
                        acc.future.complete(acc);
                    }
                }
            }
            case "stats" -> {
                final StatsAccumulator acc = pendingStats.get(cid);
                if (acc != null) {
                    final JsonNode statsArray = msg.get("stats");
                    if (statsArray != null && statsArray.isArray()) {
                        statsArray.forEach(s -> acc.stats.add(statEntry(s)));
                    }
                    acc.status = text(msg, "status");
                    if (msg.path("endOfBatch").asBoolean(false)) {
                        pendingStats.remove(cid);
                        acc.future.complete(acc);
                    }
                }
            }
            case "subscribed" -> {
                final CompletableFuture<List<String>> ack = pendingSubscribeAcks.remove(cid);
                if (ack != null) {
                    final List<String> cacheIds = new ArrayList<>();
                    final JsonNode arr = msg.get("cacheIds");
                    if (arr != null && arr.isArray()) {
                        arr.forEach(n -> cacheIds.add(n.asText()));
                    }
                    ack.complete(cacheIds);
                }
            }
            case "streamUpdate" -> dispatchStreamUpdate(cid, msg);
            case "error" -> dispatchError(cid, msg);
            default -> {
                // ignore unknown frames
            }
        }
    }

    private void dispatchStreamUpdate(String cid, JsonNode msg) {
        final String cacheId = text(msg, "cacheId");
        final List<SubRegistration> regs = subscriptions.get(cacheId);
        if (regs == null || regs.isEmpty()) {
            return;
        }
        final String eventType = text(msg, "eventType");
        final String key = text(msg, "key");
        final JsonNode value = msg.get("value");
        for (SubRegistration reg : regs) {
            if (reg.counters) {
                final CounterUpdateEvent event = new CounterUpdateEvent();
                event.setCacheId(cacheId);
                event.setEventType(eventType);
                event.setItemKey(key);
                event.setItemValue(value == null || value.isNull() ? null : value.asLong());
                event.setRequestId(cid);
                if (reg.counterListener != null) {
                    reg.counterListener.accept(event);
                }
            } else {
                final CacheUpdateEvent event = new CacheUpdateEvent();
                event.setCacheId(cacheId);
                event.setEventType(eventType);
                event.setItemKey(key);
                event.setItemValue(value == null || value.isNull() ? null : value.asText());
                event.setRequestId(cid);
                if (reg.cacheListener != null) {
                    reg.cacheListener.accept(event);
                }
            }
        }
    }

    private void dispatchError(String cid, JsonNode msg) {
        if (cid == null) {
            return;
        }
        final BidiException error = new BidiException(text(msg, "status"), text(msg, "message"));
        final CompletableFuture<JsonNode> cmd = pendingCommands.remove(cid);
        if (cmd != null) {
            cmd.completeExceptionally(error);
            return;
        }
        final EntriesAccumulator entries = pendingEntries.remove(cid);
        if (entries != null) {
            entries.future.completeExceptionally(error);
            return;
        }
        final StatsAccumulator stats = pendingStats.remove(cid);
        if (stats != null) {
            stats.future.completeExceptionally(error);
            return;
        }
        final CompletableFuture<List<String>> ack = pendingSubscribeAcks.remove(cid);
        if (ack != null) {
            ack.completeExceptionally(error);
        }
    }

    private void failAllPending(Throwable cause) {
        pendingCommands.values().forEach(f -> f.completeExceptionally(cause));
        pendingCommands.clear();
        pendingEntries.values().forEach(a -> a.future.completeExceptionally(cause));
        pendingEntries.clear();
        pendingStats.values().forEach(a -> a.future.completeExceptionally(cause));
        pendingStats.clear();
        pendingSubscribeAcks.values().forEach(f -> f.completeExceptionally(cause));
        pendingSubscribeAcks.clear();
    }

    // ------------------------------------------------------------------ WebSocket listener

    private final class BidiListener implements WebSocket.Listener {
        private final StringBuilder buffer = new StringBuilder();

        @Override
        public void onOpen(WebSocket ws) {
            ws.request(1);
        }

        @Override
        public CompletionStage<?> onText(WebSocket ws, CharSequence data, boolean last) {
            buffer.append(data);
            if (last) {
                final String raw = buffer.toString();
                buffer.setLength(0);
                try {
                    dispatch(objectMapper.readTree(raw));
                } catch (Exception e) {
                    LOG.log(System.Logger.Level.WARNING, "Failed to parse frame: {0}", raw);
                }
            }
            ws.request(1);
            return null;
        }

        @Override
        public void onError(WebSocket ws, Throwable error) {
            failAllPending(error);
        }

        @Override
        public CompletionStage<?> onClose(WebSocket ws, int statusCode, String reason) {
            failAllPending(new IllegalStateException("WebSocket closed: " + statusCode + " " + reason));
            return null;
        }
    }

    // ------------------------------------------------------------------ accumulators / registration

    private static final class EntriesAccumulator {
        final CompletableFuture<EntriesAccumulator> future = new CompletableFuture<>();
        final Map<String, JsonNode> items = new LinkedHashMap<>();
        volatile String cacheId;
        volatile String status;

        EntriesAccumulator(String cacheId) {
            this.cacheId = cacheId;
        }
    }

    private static final class StatsAccumulator {
        final CompletableFuture<StatsAccumulator> future = new CompletableFuture<>();
        final List<StatEntry> stats = new ArrayList<>();
        volatile String status;
    }

    private static final class SubRegistration {
        final Consumer<CacheUpdateEvent> cacheListener;
        final Consumer<CounterUpdateEvent> counterListener;
        final boolean counters;
        volatile String correlationId;

        SubRegistration(Consumer<CacheUpdateEvent> cacheListener, Consumer<CounterUpdateEvent> counterListener,
                        boolean counters) {
            this.cacheListener = cacheListener;
            this.counterListener = counterListener;
            this.counters = counters;
        }
    }

    // ------------------------------------------------------------------ helpers

    private static String text(JsonNode node, String field) {
        final JsonNode v = node.get(field);
        return v == null || v.isNull() ? null : v.asText();
    }

    private static StatEntry statEntry(JsonNode node) {
        return new StatEntry(
                text(node, "cacheId"),
                node.path("addedCount").asLong(0L),
                node.path("removedCount").asLong(0L),
                node.path("clearedCount").asLong(0L),
                node.path("size").asLong(0L));
    }

    private CounterResponse counterResponse(JsonNode r) {
        final JsonNode value = r.get("value");
        final CounterResponse response = new CounterResponse();
        response.setCacheId(text(r, "cacheId"));
        response.setKey(text(r, "key"));
        response.setValue(value == null || value.isNull() ? 0L : value.asLong());
        response.setOperationStatus(text(r, "status"));
        return response;
    }

    private static CreateResponse createResponse(String cacheId, String status) {
        final CreateResponse r = new CreateResponse();
        r.setCacheId(cacheId);
        r.setOperationStatus(status);
        return r;
    }

    private static PutItemResponse putItemResponse(String cacheId, String key, String status) {
        final PutItemResponse r = new PutItemResponse();
        r.setCacheId(cacheId);
        r.setKey(key);
        r.setStatus(status);
        r.setOperationStatus(status);
        return r;
    }

    private static PatchItemResponse patchItemResponse(String cacheId, String key, String status) {
        final PatchItemResponse r = new PatchItemResponse();
        r.setCacheId(cacheId);
        r.setKey(key);
        r.setOperationStatus(status);
        return r;
    }

    private static GetItemResponse getItemResponse(String cacheId, String key, String value, String status) {
        final GetItemResponse r = new GetItemResponse();
        r.setCacheId(cacheId);
        r.setKey(key);
        r.setValue(value);
        r.setOperationStatus(status);
        return r;
    }

    private static DeleteItemResponse deleteItemResponse(String cacheId, String key, String status) {
        final DeleteItemResponse r = new DeleteItemResponse();
        r.setCacheId(cacheId);
        r.setKey(key);
        r.setOperationStatus(status);
        return r;
    }

    private static CancelItemRemovalResponse cancelItemRemovalResponse(String cacheId, String key, String status) {
        final CancelItemRemovalResponse r = new CancelItemRemovalResponse();
        r.setCacheId(cacheId);
        r.setKey(key);
        r.setOperationStatus(status);
        return r;
    }

    private static ClearCacheResponse clearCacheResponse(String cacheId, String status) {
        final ClearCacheResponse r = new ClearCacheResponse();
        r.setCacheId(cacheId);
        r.setOperationStatus(status);
        return r;
    }

    private static DeleteCacheResponse deleteCacheResponse(String cacheId, String status) {
        final DeleteCacheResponse r = new DeleteCacheResponse();
        r.setCacheId(cacheId);
        r.setOperationStatus(status);
        return r;
    }

    private static ThreadFactory daemonFactory(String name) {
        return r -> {
            final Thread t = new Thread(r, name);
            t.setDaemon(true);
            return t;
        };
    }
}
