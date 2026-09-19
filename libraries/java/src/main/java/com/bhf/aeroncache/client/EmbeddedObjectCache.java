package com.bhf.aeroncache.client;

import com.bhf.aeroncache.models.*;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.Iterator;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Consumer;

/**
 * A cache view whose values are structured JSON objects rather than opaque strings, mirroring remote
 * updates into a local map. It works over either transport via {@link CacheTransport}: the HTTP+WS
 * {@link AeronCacheClient} or the Aeron {@link com.bhf.aeroncache.client.gateway.AeronGatewayClient}.
 * <p>
 * The key difference from {@link EmbeddedAeronCache} is how {@code PATCH_ITEM} events are applied: instead
 * of overwriting the entry with the delta, the delta is <em>deep-merged</em> into the stored object using
 * RFC 7386 (JSON Merge Patch) semantics &mdash; nested objects merge recursively, scalars and arrays are
 * replaced, and a {@code null} field in the delta deletes that field. This matches the server-side
 * {@link CacheTransport#patchItem patchItem} deep-merge, so a patch-mode subscription (which streams only
 * the changed fields) reconstructs the full object locally without losing untouched fields.
 * <p>
 * Values are held as Jackson {@link ObjectNode}s. Use {@link #getLocal(String)} for the raw tree or
 * {@link #getLocalAs(String, Class)} to deserialize into a POJO on demand.
 * <p>
 * Two subscription styles are offered: the transport-neutral {@link #subscribe(Consumer)} (works over
 * both transports), and the WebSocket-listener {@link #subscribe(AeronCacheSubscriber)} (HTTP only,
 * retained for backwards compatibility).
 */
public class EmbeddedObjectCache {
    private final CacheTransport transport;
    /** Non-null only when backed by the HTTP client; enables the legacy WebSocket-listener subscribe API. */
    private final AeronCacheClient httpClient;
    private final String cacheId;
    private final ObjectMapper objectMapper;
    private final Map<String, ObjectNode> localCache = new ConcurrentHashMap<>();

    public EmbeddedObjectCache(AeronCacheClient client, String cacheId) {
        this(client, cacheId, new ObjectMapper());
    }

    public EmbeddedObjectCache(CacheTransport transport, String cacheId) {
        this(transport, cacheId, new ObjectMapper());
    }

    public EmbeddedObjectCache(CacheTransport transport, String cacheId, ObjectMapper objectMapper) {
        this.transport = transport;
        this.httpClient = transport instanceof AeronCacheClient ac ? ac : null;
        this.cacheId = cacheId;
        this.objectMapper = objectMapper;
    }

    // --- Local mirror access ---

    /** The locally mirrored object for {@code key}, or {@code null} if absent. */
    public ObjectNode getLocal(String key) {
        return localCache.get(key);
    }

    /** The locally mirrored object for {@code key} deserialized into {@code type}, or {@code null} if absent. */
    public <T> T getLocalAs(String key, Class<T> type) {
        ObjectNode node = localCache.get(key);
        if (node == null) {
            return null;
        }
        try {
            return objectMapper.treeToValue(node, type);
        } catch (JsonProcessingException e) {
            throw new IllegalArgumentException("Cannot deserialize cached value for '" + key + "' into " + type, e);
        }
    }

    public Map<String, ObjectNode> getLocalCache() {
        return java.util.Collections.unmodifiableMap(localCache);
    }

    // --- Remote operations ---

    public PutItemResponse put(String key, Object value) throws Exception {
        return transport.putItem(cacheId, key, toJson(value));
    }

    public PutItemResponse putTimed(String key, Object value, long ttl) throws Exception {
        return transport.putTimedItem(cacheId, key, toJson(value), ttl);
    }

    /**
     * Deep-merge a JSON fragment into the stored object (RFC 7386). The fragment is serialized from
     * {@code fragment}; a {@code null}-valued field deletes that field.
     */
    public PatchItemResponse patch(String key, Object fragment) throws Exception {
        return transport.patchItem(cacheId, key, toJson(fragment));
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

    public CompletableFuture<PutItemResponse> putAsync(String key, Object value) {
        try {
            return transport.putItemAsync(cacheId, key, toJson(value));
        } catch (Exception e) {
            return CompletableFuture.failedFuture(e);
        }
    }

    public CompletableFuture<PutItemResponse> putTimedAsync(String key, Object value, long ttl) {
        try {
            return transport.putTimedItemAsync(cacheId, key, toJson(value), ttl);
        } catch (Exception e) {
            return CompletableFuture.failedFuture(e);
        }
    }

    public CompletableFuture<PatchItemResponse> patchAsync(String key, Object fragment) {
        try {
            return transport.patchItemAsync(cacheId, key, toJson(fragment));
        } catch (Exception e) {
            return CompletableFuture.failedFuture(e);
        }
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
        return subscribe(subscriber, hydrate, null, null);
    }

    /**
     * Subscribe over the HTTP WebSocket transport with an optional key filter and subscription mode. For
     * this object cache, {@code mode == "patch"} streams only changed fields, which are deep-merged into the
     * local objects.
     *
     * @param keys optional comma-separated key filter(s); each token is {@code cacheId:key} or a bare
     *             {@code key}. Pass {@code null} for no filtering.
     * @param mode optional subscription mode, {@code "full"} (default) or {@code "patch"}. Pass {@code null}
     *             for the server default.
     */
    public ReconnectingWebSocket subscribe(AeronCacheSubscriber subscriber, boolean hydrate, String keys, String mode) {
        if (httpClient == null) {
            throw new UnsupportedOperationException(
                    "The WebSocket-listener subscribe API requires the HTTP transport; "
                            + "use subscribe(Consumer<CacheUpdateEvent>) with the Aeron transport.");
        }
        subscriber.setInternalUpdater(this::updateLocalCache);
        return httpClient.subscribe(cacheId, hydrate, keys, mode, subscriber);
    }

    private void updateLocalCache(CacheUpdateEvent event) {
        String eventType = event.getEventType();
        String key = event.getItemKey();
        String value = event.getItemValue();

        if ("ADD_ITEM".equals(eventType)) {
            if (key != null && value != null) {
                localCache.put(key, parseObject(value));
            }
        } else if ("PATCH_ITEM".equals(eventType)) {
            if (key != null && value != null) {
                ObjectNode delta = parseObject(value);
                localCache.merge(key, delta, EmbeddedObjectCache::deepMerge);
            }
        } else if ("REMOVE_ITEM".equals(eventType)) {
            if (key != null) {
                localCache.remove(key);
            }
        } else if ("DELETE_CACHE".equals(eventType) || "CLEAR_CACHE".equals(eventType)) {
            localCache.clear();
        }
    }

    // --- helpers ---

    private String toJson(Object value) throws JsonProcessingException {
        return objectMapper.writeValueAsString(value);
    }

    private ObjectNode parseObject(String json) {
        final JsonNode node;
        try {
            node = objectMapper.readTree(json);
        } catch (JsonProcessingException e) {
            throw new IllegalArgumentException("Invalid JSON object value: " + json, e);
        }
        if (!node.isObject()) {
            throw new IllegalArgumentException("EmbeddedObjectCache values must be JSON objects, got: " + json);
        }
        return (ObjectNode) node;
    }

    /**
     * RFC 7386 (JSON Merge Patch) deep-merge of {@code patch} into {@code target}, returning a new object.
     * Nested objects merge recursively; scalars and arrays replace; a {@code null} field deletes the field.
     */
    static ObjectNode deepMerge(ObjectNode target, ObjectNode patch) {
        ObjectNode result = target.deepCopy();
        Iterator<Map.Entry<String, JsonNode>> fields = patch.fields();
        while (fields.hasNext()) {
            Map.Entry<String, JsonNode> field = fields.next();
            String name = field.getKey();
            JsonNode patchValue = field.getValue();
            if (patchValue.isNull()) {
                result.remove(name);
            } else if (patchValue.isObject()) {
                JsonNode existing = result.get(name);
                if (existing != null && existing.isObject()) {
                    result.set(name, deepMerge((ObjectNode) existing, (ObjectNode) patchValue));
                } else {
                    result.set(name, patchValue.deepCopy());
                }
            } else {
                result.set(name, patchValue.deepCopy());
            }
        }
        return result;
    }
}
