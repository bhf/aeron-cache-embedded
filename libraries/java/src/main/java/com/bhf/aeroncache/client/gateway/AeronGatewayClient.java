package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.client.CacheTransport;
import com.bhf.aeroncache.gateway.messages.OperationStatus;
import com.bhf.aeroncache.gateway.messages.UpdateEventType;
import com.bhf.aeroncache.models.CacheItem;
import com.bhf.aeroncache.models.CacheUpdateEvent;
import com.bhf.aeroncache.models.CounterUpdateEvent;
import com.bhf.aeroncache.models.CreateResponse;
import com.bhf.aeroncache.models.DeleteCacheResponse;
import com.bhf.aeroncache.models.DeleteItemResponse;
import com.bhf.aeroncache.models.GetCacheResponse;
import com.bhf.aeroncache.models.GetItemResponse;
import com.bhf.aeroncache.models.CounterResponse;
import com.bhf.aeroncache.models.ClearCacheResponse;
import com.bhf.aeroncache.models.PutItemResponse;
import io.aeron.Aeron;
import org.agrona.CloseHelper;
import org.agrona.concurrent.AgentRunner;
import org.agrona.concurrent.BackoffIdleStrategy;
import org.agrona.concurrent.IdleStrategy;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ThreadFactory;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.locks.LockSupport;
import java.util.function.Consumer;

/**
 * High-level client for the Aeron Cache gateway — the Aeron-transport counterpart to
 * {@link com.bhf.aeroncache.client.AeronCacheClient} (HTTP+WS).
 * <p>
 * It owns the {@link GatewayClient} agent thread and presents a futures-based API mirroring the HTTP
 * client's method names and response models: each command mints a correlation id, registers a pending
 * {@link CompletableFuture}, and completes it when the correlated {@code GatewayCommandResponse} (or
 * {@code GatewayError}) arrives. Synchronous variants simply block on the future.
 * <p>
 * Cache and counter operations both ride the single bidirectional Aeron connection; subscriptions deliver
 * {@link CacheUpdateEvent}/{@link CounterUpdateEvent} callbacks over the same connection.
 */
public class AeronGatewayClient implements CacheTransport, AutoCloseable {

    /** Default gateway request endpoint port (see the server's GatewayApplication). */
    public static final int DEFAULT_REQUEST_PORT = 7075;
    /** Default gateway response control endpoint port. */
    public static final int DEFAULT_RESPONSE_CONTROL_PORT = 7076;
    /** Default gateway request stream id. */
    public static final int DEFAULT_REQUEST_STREAM_ID = 100;
    /** Default gateway response stream id. */
    public static final int DEFAULT_RESPONSE_STREAM_ID = 101;

    private static final long DEFAULT_REQUEST_TIMEOUT_MS = 5_000L;

    private final Aeron aeron;
    private final boolean ownsAeron;
    private final GatewayClient gatewayClient;
    private final AgentRunner agentRunner;
    private final long requestTimeoutMs;

    private final ScheduledExecutorService timeouts =
            Executors.newSingleThreadScheduledExecutor(daemonFactory("aeron-gateway-timeouts"));

    // correlationId -> pending single-response command
    private final Map<String, PendingCommand<?>> pendingCommands = new ConcurrentHashMap<>();
    // correlationId -> accumulating getEntries request
    private final Map<String, EntriesAccumulator> pendingEntries = new ConcurrentHashMap<>();
    // correlationId -> accumulating getStats request
    private final Map<String, StatsAccumulator> pendingStats = new ConcurrentHashMap<>();

    // cacheId -> subscription listeners
    private final Map<String, List<Consumer<CacheUpdateEvent>>> cacheListeners = new ConcurrentHashMap<>();
    private final Map<String, List<Consumer<CounterUpdateEvent>>> counterListeners = new ConcurrentHashMap<>();

    // ------------------------------------------------------------------ construction

    /**
     * Connect using an existing {@link Aeron} instance and explicit endpoints/streams. The Aeron
     * instance is <em>not</em> owned by this client and will not be closed by {@link #close()}.
     */
    public AeronGatewayClient(Aeron aeron,
                              String requestEndpoint,
                              int requestStreamId,
                              String responseControl,
                              int responseStreamId) {
        this(aeron, false, requestEndpoint, requestStreamId, responseControl, responseStreamId, DEFAULT_REQUEST_TIMEOUT_MS);
    }

    private AeronGatewayClient(Aeron aeron,
                               boolean ownsAeron,
                               String requestEndpoint,
                               int requestStreamId,
                               String responseControl,
                               int responseStreamId,
                               long requestTimeoutMs) {
        this.aeron = aeron;
        this.ownsAeron = ownsAeron;
        this.requestTimeoutMs = requestTimeoutMs;
        this.gatewayClient = new GatewayClient(aeron, requestEndpoint, requestStreamId,
                responseControl, responseStreamId, new DispatchingListener());
        final IdleStrategy idleStrategy = new BackoffIdleStrategy();
        this.agentRunner = new AgentRunner(idleStrategy, Throwable::printStackTrace, null, gatewayClient);
        AgentRunner.startOnThread(agentRunner);
    }

    /**
     * Connect to a gateway on the given host using the default ports and stream ids, creating and owning
     * an {@link Aeron} instance bound to {@code aeronDir}.
     *
     * @param aeronDir the media driver's aeron directory (see {@code aeron.dir}/{@code AERON_DIR}).
     * @param host     the gateway host.
     */
    public static AeronGatewayClient connect(String aeronDir, String host) {
        return connect(aeronDir, host + ":" + DEFAULT_REQUEST_PORT, DEFAULT_REQUEST_STREAM_ID,
                host + ":" + DEFAULT_RESPONSE_CONTROL_PORT, DEFAULT_RESPONSE_STREAM_ID);
    }

    /**
     * Connect with fully explicit endpoints/streams, creating and owning an {@link Aeron} instance bound
     * to {@code aeronDir}. The created Aeron instance is closed by {@link #close()}.
     */
    public static AeronGatewayClient connect(String aeronDir,
                                             String requestEndpoint,
                                             int requestStreamId,
                                             String responseControl,
                                             int responseStreamId) {
        final Aeron.Context ctx = new Aeron.Context();
        if (aeronDir != null && !aeronDir.isEmpty()) {
            ctx.aeronDirectoryName(aeronDir);
        }
        final Aeron aeron = Aeron.connect(ctx);
        return new AeronGatewayClient(aeron, true, requestEndpoint, requestStreamId,
                responseControl, responseStreamId, DEFAULT_REQUEST_TIMEOUT_MS);
    }

    /**
     * Block until both the request publication and response subscription are connected, or the timeout
     * elapses.
     * <p>
     * The gateway creates each client's response publication lazily, on receipt of that client's first
     * request frame, so the response subscription cannot connect until the client sends something.
     * This repeatedly sends a harmless warmup probe ({@code getStats}, whose response is ignored) until
     * both channels are up — matching the gateway's reference client. Callers should await this before
     * issuing real commands, otherwise the first commands may be sent before the response channel is up
     * and their responses dropped.
     *
     * @return {@code true} if fully connected within the timeout.
     */
    public boolean awaitConnected(long timeout, TimeUnit unit) {
        final long deadline = System.nanoTime() + unit.toNanos(timeout);
        while (System.nanoTime() < deadline) {
            if (gatewayClient.isConnected()) {
                return true;
            }
            // Fire-and-forget probe; its response has no pending entry and is ignored by the dispatcher.
            gatewayClient.getStats("connection-warmup");
            LockSupport.parkNanos(TimeUnit.MILLISECONDS.toNanos(250));
        }
        return gatewayClient.isConnected();
    }

    /**
     * @return {@code true} once the request publication and response subscription are both fully
     * connected (the latter after the first command has been sent).
     */
    public boolean isConnected() {
        return gatewayClient.isConnected();
    }

    /**
     * @return {@code true} once the client can send requests (request publication connected).
     */
    public boolean isReadyToSend() {
        return gatewayClient.isReadyToSend();
    }

    // ------------------------------------------------------------------ cache commands

    public CompletableFuture<CreateResponse> createCacheAsync(String cacheId) {
        return command(cid -> gatewayClient.createCache(cid, cacheId),
                (status, cId, key, value) -> createResponse(cId, status));
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
        return command(cid -> gatewayClient.addEntry(cid, cacheId, key, value, ttl),
                (status, cId, k, v) -> putItemResponse(cId, k, status));
    }

    public PutItemResponse putTimedItem(String cacheId, String key, String value, long ttl) throws Exception {
        return await(putTimedItemAsync(cacheId, key, value, ttl));
    }

    public CompletableFuture<GetItemResponse> getItemAsync(String cacheId, String key) {
        return command(cid -> gatewayClient.getEntry(cid, cacheId, key),
                (status, cId, k, v) -> getItemResponse(cId, k, v, status));
    }

    public GetItemResponse getItem(String cacheId, String key) throws Exception {
        return await(getItemAsync(cacheId, key));
    }

    public CompletableFuture<DeleteItemResponse> deleteItemAsync(String cacheId, String key) {
        return command(cid -> gatewayClient.removeEntry(cid, cacheId, key),
                (status, cId, k, v) -> deleteItemResponse(cId, k, status));
    }

    public DeleteItemResponse deleteItem(String cacheId, String key) throws Exception {
        return await(deleteItemAsync(cacheId, key));
    }

    public CompletableFuture<DeleteCacheResponse> deleteCacheAsync(String cacheId) {
        return command(cid -> gatewayClient.deleteCache(cid, cacheId),
                (status, cId, k, v) -> deleteCacheResponse(cId, status));
    }

    public DeleteCacheResponse deleteCache(String cacheId) throws Exception {
        return await(deleteCacheAsync(cacheId));
    }

    public CompletableFuture<ClearCacheResponse> clearCacheAsync(String cacheId) {
        return command(cid -> gatewayClient.clearCache(cid, cacheId),
                (status, cId, k, v) -> clearCacheResponse(cId, status));
    }

    public ClearCacheResponse clearCache(String cacheId) throws Exception {
        return await(clearCacheAsync(cacheId));
    }

    public CompletableFuture<GetCacheResponse> getCacheItemsAsync(String cacheId) {
        final String cid = UUID.randomUUID().toString();
        final CompletableFuture<GetCacheResponse> future = new CompletableFuture<>();
        pendingEntries.put(cid, new EntriesAccumulator(future));
        scheduleTimeout(cid, future);
        offer(() -> gatewayClient.getEntries(cid, cacheId), future);
        return future;
    }

    public GetCacheResponse getCacheItems(String cacheId) throws Exception {
        return await(getCacheItemsAsync(cacheId));
    }

    public CompletableFuture<List<GatewayStat>> getStatsAsync() {
        final String cid = UUID.randomUUID().toString();
        final CompletableFuture<List<GatewayStat>> future = new CompletableFuture<>();
        pendingStats.put(cid, new StatsAccumulator(future));
        scheduleTimeout(cid, future);
        offer(() -> gatewayClient.getStats(cid), future);
        return future;
    }

    public List<GatewayStat> getStats() throws Exception {
        return await(getStatsAsync());
    }

    // ------------------------------------------------------------------ counter commands

    public CompletableFuture<CreateResponse> createCounterCacheAsync(String cacheId) {
        return command(cid -> gatewayClient.createCounterCache(cid, cacheId),
                (status, cId, k, v) -> createResponse(cId, status));
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
        return command(cid -> gatewayClient.addCounterEntry(cid, cacheId, key, value, ttl),
                (status, cId, k, v) -> putItemResponse(cId, k, status));
    }

    public PutItemResponse putTimedCounter(String cacheId, String key, long value, long ttl) throws Exception {
        return await(putTimedCounterAsync(cacheId, key, value, ttl));
    }

    public CompletableFuture<CounterResponse> getCounterAsync(String cacheId, String key) {
        return command(cid -> gatewayClient.getCounterEntry(cid, cacheId, key),
                (status, cId, k, v) -> counterResponse(cId, k, v, status));
    }

    public CounterResponse getCounter(String cacheId, String key) throws Exception {
        return await(getCounterAsync(cacheId, key));
    }

    public CompletableFuture<DeleteItemResponse> deleteCounterAsync(String cacheId, String key) {
        return command(cid -> gatewayClient.removeCounterEntry(cid, cacheId, key),
                (status, cId, k, v) -> deleteItemResponse(cId, k, status));
    }

    public DeleteItemResponse deleteCounter(String cacheId, String key) throws Exception {
        return await(deleteCounterAsync(cacheId, key));
    }

    public CompletableFuture<DeleteCacheResponse> deleteCounterCacheAsync(String cacheId) {
        return command(cid -> gatewayClient.deleteCounterCache(cid, cacheId),
                (status, cId, k, v) -> deleteCacheResponse(cId, status));
    }

    public DeleteCacheResponse deleteCounterCache(String cacheId) throws Exception {
        return await(deleteCounterCacheAsync(cacheId));
    }

    public CompletableFuture<CounterResponse> incrementCounterAsync(String cacheId, String key, long amount) {
        return command(cid -> gatewayClient.incrementCounter(cid, cacheId, key, amount, 0L),
                (status, cId, k, v) -> counterResponse(cId, k, v, status));
    }

    public CounterResponse incrementCounter(String cacheId, String key, long amount) throws Exception {
        return await(incrementCounterAsync(cacheId, key, amount));
    }

    public CompletableFuture<CounterResponse> decrementCounterAsync(String cacheId, String key, long amount) {
        return command(cid -> gatewayClient.decrementCounter(cid, cacheId, key, amount, 0L),
                (status, cId, k, v) -> counterResponse(cId, k, v, status));
    }

    public CounterResponse decrementCounter(String cacheId, String key, long amount) throws Exception {
        return await(decrementCounterAsync(cacheId, key, amount));
    }

    public CompletableFuture<CounterResponse> setCounterAsync(String cacheId, String key, long value) {
        return command(cid -> gatewayClient.setCounter(cid, cacheId, key, value, 0L),
                (status, cId, k, v) -> counterResponse(cId, k, v, status));
    }

    public CounterResponse setCounter(String cacheId, String key, long value) throws Exception {
        return await(setCounterAsync(cacheId, key, value));
    }

    // ------------------------------------------------------------------ subscriptions

    /**
     * Subscribe to streaming updates for a regular cache. Updates are delivered as
     * {@link CacheUpdateEvent} on the client's agent thread, so the listener must not block.
     *
     * @return an {@link AutoCloseable} handle; closing it unsubscribes.
     */
    public GatewaySubscription subscribe(String cacheId, Consumer<CacheUpdateEvent> listener) {
        return subscribe(cacheId, false, listener);
    }

    public GatewaySubscription subscribe(String cacheId, boolean sendSnapshot, Consumer<CacheUpdateEvent> listener) {
        cacheListeners.computeIfAbsent(cacheId, k -> new CopyOnWriteArrayList<>()).add(listener);
        final String cid = UUID.randomUUID().toString();
        offer(() -> gatewayClient.subscribe(cid, List.of(cacheId), sendSnapshot, false), null);
        return new GatewaySubscription(cacheId, false, () -> {
            removeListener(cacheListeners, cacheId, listener);
            offer(() -> gatewayClient.unsubscribe(UUID.randomUUID().toString(), cacheId, false), null);
        });
    }

    /**
     * Subscribe to streaming updates for a counter cache. Updates are delivered as
     * {@link CounterUpdateEvent} on the client's agent thread.
     *
     * @return an {@link AutoCloseable} handle; closing it unsubscribes.
     */
    public GatewaySubscription subscribeCounter(String cacheId, Consumer<CounterUpdateEvent> listener) {
        return subscribeCounter(cacheId, false, listener);
    }

    public GatewaySubscription subscribeCounter(String cacheId, boolean sendSnapshot, Consumer<CounterUpdateEvent> listener) {
        counterListeners.computeIfAbsent(cacheId, k -> new CopyOnWriteArrayList<>()).add(listener);
        final String cid = UUID.randomUUID().toString();
        offer(() -> gatewayClient.subscribe(cid, List.of(cacheId), sendSnapshot, true), null);
        return new GatewaySubscription(cacheId, true, () -> {
            removeListener(counterListeners, cacheId, listener);
            offer(() -> gatewayClient.unsubscribe(UUID.randomUUID().toString(), cacheId, true), null);
        });
    }

    // --- Transport-neutral subscriptions (CacheTransport) ---

    @Override
    public AutoCloseable subscribeCacheUpdates(String cacheId, boolean hydrate, Consumer<CacheUpdateEvent> listener) {
        return subscribe(cacheId, hydrate, listener);
    }

    @Override
    public AutoCloseable subscribeCounterUpdates(String cacheId, boolean hydrate, Consumer<CounterUpdateEvent> listener) {
        return subscribeCounter(cacheId, hydrate, listener);
    }

    // ------------------------------------------------------------------ lifecycle

    @Override
    public void close() {
        timeouts.shutdownNow();
        CloseHelper.quietClose(agentRunner);
        if (ownsAeron) {
            CloseHelper.quietClose(aeron);
        }
        failAllPending(new IllegalStateException("AeronGatewayClient closed"));
    }

    // ------------------------------------------------------------------ internals

    /** Maps a decoded gateway command response onto a response model of type {@code T}. */
    @FunctionalInterface
    private interface ResponseMapper<T> {
        T map(OperationStatus status, String cacheId, String key, String value);
    }

    private record PendingCommand<T>(CompletableFuture<T> future, ResponseMapper<T> mapper) {
    }

    private final class EntriesAccumulator {
        final CompletableFuture<GetCacheResponse> future;
        final List<CacheItem> items = new ArrayList<>();
        volatile String cacheId;
        volatile OperationStatus status;

        EntriesAccumulator(CompletableFuture<GetCacheResponse> future) {
            this.future = future;
        }
    }

    private final class StatsAccumulator {
        final CompletableFuture<List<GatewayStat>> future;
        final List<GatewayStat> stats = new ArrayList<>();

        StatsAccumulator(CompletableFuture<List<GatewayStat>> future) {
            this.future = future;
        }
    }

    private <T> CompletableFuture<T> command(java.util.function.Function<String, Long> send, ResponseMapper<T> mapper) {
        final String cid = UUID.randomUUID().toString();
        final CompletableFuture<T> future = new CompletableFuture<>();
        pendingCommands.put(cid, new PendingCommand<>(future, mapper));
        scheduleTimeout(cid, future);
        offer(() -> send.apply(cid), future);
        return future;
    }

    /**
     * Offer a request frame to the gateway client, retrying briefly on ring-buffer backpressure. If the
     * offer never succeeds within the request timeout, the future (if any) is failed.
     */
    private void offer(java.util.function.LongSupplier send, CompletableFuture<?> future) {
        final long deadline = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(requestTimeoutMs);
        while (true) {
            if (send.getAsLong() != Aeron.NULL_VALUE) {
                return;
            }
            if (System.nanoTime() > deadline) {
                if (future != null) {
                    future.completeExceptionally(new TimeoutException("Timed out enqueuing request (backpressure)"));
                }
                return;
            }
            LockSupport.parkNanos(TimeUnit.MICROSECONDS.toNanos(100));
        }
    }

    private void scheduleTimeout(String correlationId, CompletableFuture<?> future) {
        timeouts.schedule(() -> {
            if (!future.isDone()) {
                pendingCommands.remove(correlationId);
                pendingEntries.remove(correlationId);
                pendingStats.remove(correlationId);
                future.completeExceptionally(new TimeoutException("Gateway request timed out: " + correlationId));
            }
        }, requestTimeoutMs, TimeUnit.MILLISECONDS);
    }

    private <T> T await(CompletableFuture<T> future) throws Exception {
        try {
            return future.get(requestTimeoutMs * 2, TimeUnit.MILLISECONDS);
        } catch (java.util.concurrent.ExecutionException e) {
            final Throwable cause = e.getCause();
            if (cause instanceof Exception ex) {
                throw ex;
            }
            throw e;
        }
    }

    private static <E> void removeListener(Map<String, List<E>> map, String cacheId, E listener) {
        final List<E> list = map.get(cacheId);
        if (list != null) {
            list.remove(listener);
            if (list.isEmpty()) {
                map.remove(cacheId, list);
            }
        }
    }

    private void failAllPending(Throwable cause) {
        pendingCommands.values().forEach(p -> p.future().completeExceptionally(cause));
        pendingCommands.clear();
        pendingEntries.values().forEach(a -> a.future.completeExceptionally(cause));
        pendingEntries.clear();
        pendingStats.values().forEach(a -> a.future.completeExceptionally(cause));
        pendingStats.clear();
    }

    private static ThreadFactory daemonFactory(String name) {
        return r -> {
            final Thread t = new Thread(r, name);
            t.setDaemon(true);
            return t;
        };
    }

    // ------------------------------------------------------------------ response model builders

    private static CreateResponse createResponse(String cacheId, OperationStatus status) {
        final CreateResponse r = new CreateResponse();
        r.setCacheId(cacheId);
        r.setOperationStatus(status.name());
        return r;
    }

    private static PutItemResponse putItemResponse(String cacheId, String key, OperationStatus status) {
        final PutItemResponse r = new PutItemResponse();
        r.setCacheId(cacheId);
        r.setKey(key);
        r.setStatus(status.name());
        r.setOperationStatus(status.name());
        return r;
    }

    private static GetItemResponse getItemResponse(String cacheId, String key, String value, OperationStatus status) {
        final GetItemResponse r = new GetItemResponse();
        r.setCacheId(cacheId);
        r.setKey(key);
        r.setValue(value);
        r.setOperationStatus(status.name());
        return r;
    }

    private static DeleteItemResponse deleteItemResponse(String cacheId, String key, OperationStatus status) {
        final DeleteItemResponse r = new DeleteItemResponse();
        r.setCacheId(cacheId);
        r.setKey(key);
        r.setOperationStatus(status.name());
        return r;
    }

    private static DeleteCacheResponse deleteCacheResponse(String cacheId, OperationStatus status) {
        final DeleteCacheResponse r = new DeleteCacheResponse();
        r.setCacheId(cacheId);
        r.setOperationStatus(status.name());
        return r;
    }

    private static ClearCacheResponse clearCacheResponse(String cacheId, OperationStatus status) {
        final ClearCacheResponse r = new ClearCacheResponse();
        r.setCacheId(cacheId);
        r.setOperationStatus(status.name());
        return r;
    }

    private static CounterResponse counterResponse(String cacheId, String key, String value, OperationStatus status) {
        final CounterResponse r = new CounterResponse();
        r.setCacheId(cacheId);
        r.setKey(key);
        r.setValue(parseLong(value));
        r.setOperationStatus(status.name());
        return r;
    }

    private static long parseLong(String value) {
        if (value == null || value.isEmpty()) {
            return 0L;
        }
        try {
            return Long.parseLong(value);
        } catch (NumberFormatException e) {
            return 0L;
        }
    }

    // ------------------------------------------------------------------ listener dispatch

    /**
     * Dispatches gateway frames (on the agent thread) to the pending futures and subscription listeners.
     */
    private final class DispatchingListener implements GatewayClientListener {

        @Override
        @SuppressWarnings("unchecked")
        public void onCommandResponse(String correlationId, OperationStatus status, String cacheId, String key, String value) {
            final PendingCommand<?> pending = pendingCommands.remove(correlationId);
            if (pending == null) {
                return;
            }
            try {
                final Object result = ((ResponseMapper<Object>) pending.mapper()).map(status, cacheId, key, value);
                ((CompletableFuture<Object>) pending.future()).complete(result);
            } catch (Exception e) {
                pending.future().completeExceptionally(e);
            }
        }

        @Override
        public void onEntries(String correlationId, OperationStatus status, String cacheId, Map<String, String> items, boolean endOfBatch) {
            final EntriesAccumulator acc = pendingEntries.get(correlationId);
            if (acc == null) {
                return;
            }
            acc.status = status;
            acc.cacheId = cacheId;
            items.forEach((k, v) -> acc.items.add(new CacheItem(k, v)));
            if (endOfBatch) {
                pendingEntries.remove(correlationId);
                final GetCacheResponse response = new GetCacheResponse();
                response.setCacheId(cacheId);
                response.setOperationStatus(status.name());
                response.setItems(acc.items);
                acc.future.complete(response);
            }
        }

        @Override
        public void onStats(String correlationId, OperationStatus status, List<GatewayStat> stats, boolean endOfBatch) {
            final StatsAccumulator acc = pendingStats.get(correlationId);
            if (acc == null) {
                return;
            }
            acc.stats.addAll(stats);
            if (endOfBatch) {
                pendingStats.remove(correlationId);
                acc.future.complete(acc.stats);
            }
        }

        @Override
        public void onStreamUpdate(String correlationId, UpdateEventType eventType, String cacheId, String key, String value) {
            final List<Consumer<CacheUpdateEvent>> cacheSubs = cacheListeners.get(cacheId);
            if (cacheSubs != null && !cacheSubs.isEmpty()) {
                final CacheUpdateEvent event = new CacheUpdateEvent();
                event.setCacheId(cacheId);
                event.setEventType(eventType.name());
                event.setItemKey(key);
                event.setItemValue(value);
                event.setRequestId(correlationId);
                for (Consumer<CacheUpdateEvent> sub : cacheSubs) {
                    sub.accept(event);
                }
            }
            final List<Consumer<CounterUpdateEvent>> counterSubs = counterListeners.get(cacheId);
            if (counterSubs != null && !counterSubs.isEmpty()) {
                final CounterUpdateEvent event = new CounterUpdateEvent();
                event.setCacheId(cacheId);
                event.setEventType(eventType.name());
                event.setItemKey(key);
                event.setItemValue(value == null || value.isEmpty() ? null : parseLong(value));
                event.setRequestId(correlationId);
                for (Consumer<CounterUpdateEvent> sub : counterSubs) {
                    sub.accept(event);
                }
            }
        }

        @Override
        public void onError(String correlationId, OperationStatus status, String message) {
            final GatewayException error = new GatewayException(status, message);
            final PendingCommand<?> pending = pendingCommands.remove(correlationId);
            if (pending != null) {
                pending.future().completeExceptionally(error);
                return;
            }
            final EntriesAccumulator entries = pendingEntries.remove(correlationId);
            if (entries != null) {
                entries.future.completeExceptionally(error);
                return;
            }
            final StatsAccumulator stats = pendingStats.remove(correlationId);
            if (stats != null) {
                stats.future.completeExceptionally(error);
            }
        }
    }
}
