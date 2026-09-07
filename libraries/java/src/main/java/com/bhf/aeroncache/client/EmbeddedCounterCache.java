package com.bhf.aeroncache.client;

import com.bhf.aeroncache.models.*;

import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Consumer;

/**
 * A counter cache view that mirrors remote updates into a local map. It works over either transport via
 * {@link CacheTransport}: the HTTP+WS {@link AeronCacheClient} or the Aeron
 * {@link com.bhf.aeroncache.client.gateway.AeronGatewayClient}.
 * <p>
 * Two subscription styles are offered: the transport-neutral {@link #subscribe(Consumer)} (works over
 * both transports), and the WebSocket-listener {@link #subscribe(CounterCacheSubscriber)} (HTTP only,
 * retained for backwards compatibility).
 */
public class EmbeddedCounterCache {
    private final CacheTransport transport;
    /** Non-null only when backed by the HTTP client; enables the legacy WebSocket-listener subscribe API. */
    private final AeronCacheClient httpClient;
    private final String cacheId;
    private final Map<String, Long> localCache = new ConcurrentHashMap<>();

    public EmbeddedCounterCache(AeronCacheClient client, String cacheId) {
        this.transport = client;
        this.httpClient = client;
        this.cacheId = cacheId;
    }

    public EmbeddedCounterCache(CacheTransport transport, String cacheId) {
        this.transport = transport;
        this.httpClient = transport instanceof AeronCacheClient ac ? ac : null;
        this.cacheId = cacheId;
    }

    public Long getLocal(String key) {
        return localCache.get(key);
    }

    public Map<String, Long> getLocalCache() {
        return java.util.Collections.unmodifiableMap(localCache);
    }

    public PutItemResponse put(String key, long value) throws Exception {
        return transport.putCounter(cacheId, key, value);
    }

    public PutItemResponse putTimed(String key, long value, long ttl) throws Exception {
        return transport.putTimedCounter(cacheId, key, value, ttl);
    }

    public CounterResponse get(String key) throws Exception {
        return transport.getCounter(cacheId, key);
    }

    public CounterResponse increment(String key, long amount) throws Exception {
        return transport.incrementCounter(cacheId, key, amount);
    }

    public CounterResponse decrement(String key, long amount) throws Exception {
        return transport.decrementCounter(cacheId, key, amount);
    }

    public CounterResponse set(String key, long value) throws Exception {
        return transport.setCounter(cacheId, key, value);
    }

    public DeleteItemResponse remove(String key) throws Exception {
        return transport.deleteCounter(cacheId, key);
    }

    public DeleteCacheResponse clear() throws Exception {
        return transport.deleteCounterCache(cacheId);
    }

    public CompletableFuture<PutItemResponse> putAsync(String key, long value) {
        return transport.putCounterAsync(cacheId, key, value);
    }

    public CompletableFuture<PutItemResponse> putTimedAsync(String key, long value, long ttl) {
        return transport.putTimedCounterAsync(cacheId, key, value, ttl);
    }

    public CompletableFuture<CounterResponse> getAsync(String key) {
        return transport.getCounterAsync(cacheId, key);
    }

    public CompletableFuture<CounterResponse> incrementAsync(String key, long amount) {
        return transport.incrementCounterAsync(cacheId, key, amount);
    }

    public CompletableFuture<CounterResponse> decrementAsync(String key, long amount) {
        return transport.decrementCounterAsync(cacheId, key, amount);
    }

    public CompletableFuture<CounterResponse> setAsync(String key, long value) {
        return transport.setCounterAsync(cacheId, key, value);
    }

    public CompletableFuture<DeleteItemResponse> removeAsync(String key) {
        return transport.deleteCounterAsync(cacheId, key);
    }

    public CompletableFuture<DeleteCacheResponse> clearAsync() {
        return transport.deleteCounterCacheAsync(cacheId);
    }

    // --- Transport-neutral subscription (works over HTTP+WS and Aeron) ---

    /**
     * Subscribe to counter updates, mirroring them into the local map and forwarding each event to
     * {@code listener}.
     *
     * @return a handle that unsubscribes when {@link AutoCloseable#close() closed}.
     */
    public AutoCloseable subscribe(Consumer<CounterUpdateEvent> listener) {
        return subscribe(listener, false);
    }

    public AutoCloseable subscribe(Consumer<CounterUpdateEvent> listener, boolean hydrate) {
        return transport.subscribeCounterUpdates(cacheId, hydrate, event -> {
            updateLocalCache(event);
            if (listener != null) {
                listener.accept(event);
            }
        });
    }

    // --- WebSocket-listener subscription (HTTP transport only, retained for compatibility) ---

    public ReconnectingWebSocket subscribe(CounterCacheSubscriber subscriber) {
        return subscribe(subscriber, false);
    }

    public ReconnectingWebSocket subscribe(CounterCacheSubscriber subscriber, boolean hydrate) {
        if (httpClient == null) {
            throw new UnsupportedOperationException(
                    "The WebSocket-listener subscribe API requires the HTTP transport; "
                            + "use subscribe(Consumer<CounterUpdateEvent>) with the Aeron transport.");
        }
        subscriber.setInternalUpdater(this::updateLocalCache);
        return httpClient.subscribeCounter(cacheId, hydrate, subscriber);
    }

    private void updateLocalCache(CounterUpdateEvent event) {
        String eventType = event.getEventType();
        String key = event.getItemKey();
        Long value = event.getItemValue();

        if ("ADD_ITEM".equals(eventType)) {
            if (key != null && value != null) {
                localCache.put(key, value);
            }
        } else if ("REMOVE_ITEM".equals(eventType)) {
            if (key != null) {
                localCache.remove(key);
            }
        } else if ("DELETE_CACHE".equals(eventType) || "CLEAR_CACHE".equals(eventType)) {
            localCache.clear();
        }
    }
}
