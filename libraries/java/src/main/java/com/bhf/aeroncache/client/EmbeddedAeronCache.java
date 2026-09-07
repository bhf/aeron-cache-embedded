package com.bhf.aeroncache.client;

import com.bhf.aeroncache.models.*;

import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Consumer;

/**
 * A cache view that mirrors remote updates into a local map. It works over either transport via
 * {@link CacheTransport}: the HTTP+WS {@link AeronCacheClient} or the Aeron
 * {@link com.bhf.aeroncache.client.gateway.AeronGatewayClient}.
 * <p>
 * Two subscription styles are offered: the transport-neutral {@link #subscribe(Consumer)} (works over
 * both transports), and the WebSocket-listener {@link #subscribe(AeronCacheSubscriber)} (HTTP only,
 * retained for backwards compatibility).
 */
public class EmbeddedAeronCache {
    private final CacheTransport transport;
    /** Non-null only when backed by the HTTP client; enables the legacy WebSocket-listener subscribe API. */
    private final AeronCacheClient httpClient;
    private final String cacheId;
    private final Map<String, String> localCache = new ConcurrentHashMap<>();

    public EmbeddedAeronCache(AeronCacheClient client, String cacheId) {
        this.transport = client;
        this.httpClient = client;
        this.cacheId = cacheId;
    }

    public EmbeddedAeronCache(CacheTransport transport, String cacheId) {
        this.transport = transport;
        this.httpClient = transport instanceof AeronCacheClient ac ? ac : null;
        this.cacheId = cacheId;
    }

    public String getLocal(String key) {
        return localCache.get(key);
    }

    public Map<String, String> getLocalCache() {
        return java.util.Collections.unmodifiableMap(localCache);
    }

    public PutItemResponse put(String key, String value) throws Exception {
        return transport.putItem(cacheId, key, value);
    }

    public PutItemResponse putTimed(String key, String value, long ttl) throws Exception {
        return transport.putTimedItem(cacheId, key, value, ttl);
    }

    public GetItemResponse get(String key) throws Exception {
        return transport.getItem(cacheId, key);
    }

    public DeleteItemResponse remove(String key) throws Exception {
        return transport.deleteItem(cacheId, key);
    }

    public DeleteCacheResponse clear() throws Exception {
        return transport.deleteCache(cacheId);
    }

    public CompletableFuture<PutItemResponse> putAsync(String key, String value) {
        return transport.putItemAsync(cacheId, key, value);
    }

    public CompletableFuture<PutItemResponse> putTimedAsync(String key, String value, long ttl) {
        return transport.putTimedItemAsync(cacheId, key, value, ttl);
    }

    public CompletableFuture<GetItemResponse> getAsync(String key) {
        return transport.getItemAsync(cacheId, key);
    }

    public CompletableFuture<DeleteItemResponse> removeAsync(String key) {
        return transport.deleteItemAsync(cacheId, key);
    }

    public CompletableFuture<DeleteCacheResponse> clearAsync() {
        return transport.deleteCacheAsync(cacheId);
    }

    // --- Transport-neutral subscription (works over HTTP+WS and Aeron) ---

    /**
     * Subscribe to updates, mirroring them into the local map and forwarding each event to {@code listener}.
     *
     * @return a handle that unsubscribes when {@link AutoCloseable#close() closed}.
     */
    public AutoCloseable subscribe(Consumer<CacheUpdateEvent> listener) {
        return subscribe(listener, false);
    }

    public AutoCloseable subscribe(Consumer<CacheUpdateEvent> listener, boolean hydrate) {
        return transport.subscribeCacheUpdates(cacheId, hydrate, event -> {
            updateLocalCache(event);
            if (listener != null) {
                listener.accept(event);
            }
        });
    }

    // --- WebSocket-listener subscription (HTTP transport only, retained for compatibility) ---

    public ReconnectingWebSocket subscribe(AeronCacheSubscriber subscriber) {
        return subscribe(subscriber, false);
    }

    public ReconnectingWebSocket subscribe(AeronCacheSubscriber subscriber, boolean hydrate) {
        if (httpClient == null) {
            throw new UnsupportedOperationException(
                    "The WebSocket-listener subscribe API requires the HTTP transport; "
                            + "use subscribe(Consumer<CacheUpdateEvent>) with the Aeron transport.");
        }
        subscriber.setInternalUpdater(this::updateLocalCache);
        return httpClient.subscribe(cacheId, hydrate, subscriber);
    }

    private void updateLocalCache(CacheUpdateEvent event) {
        String eventType = event.getEventType();
        String key = event.getItemKey();
        String value = event.getItemValue();

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
