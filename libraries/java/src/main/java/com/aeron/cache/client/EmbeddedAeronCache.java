package com.aeron.cache.client;

import com.aeron.cache.models.*;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.net.http.WebSocket;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionStage;
import java.util.concurrent.ConcurrentHashMap;

public class EmbeddedAeronCache {
    private final AeronCacheClient client;
    private final String cacheId;
    private final Map<String, String> localCache = new ConcurrentHashMap<>();
    private final ObjectMapper objectMapper = new ObjectMapper();

    public EmbeddedAeronCache(AeronCacheClient client, String cacheId) {
        this.client = client;
        this.cacheId = cacheId;
    }

    public String getLocal(String key) {
        return localCache.get(key);
    }

    public Map<String, String> getLocalCache() {
        return java.util.Collections.unmodifiableMap(localCache);
    }

    public PutItemResponse put(String key, String value) throws Exception {
        return client.putItem(cacheId, key, value);
    }

    public GetItemResponse get(String key) throws Exception {
        return client.getItem(cacheId, key);
    }

    public DeleteItemResponse remove(String key) throws Exception {
        return client.deleteItem(cacheId, key);
    }

    public DeleteCacheResponse clear() throws Exception {
        return client.deleteCache(cacheId);
    }

    public CompletableFuture<PutItemResponse> putAsync(String key, String value) {
        return client.putItemAsync(cacheId, key, value);
    }

    public CompletableFuture<GetItemResponse> getAsync(String key) {
        return client.getItemAsync(cacheId, key);
    }

    public CompletableFuture<DeleteItemResponse> removeAsync(String key) {
        return client.deleteItemAsync(cacheId, key);
    }

    public CompletableFuture<DeleteCacheResponse> clearAsync() {
        return client.deleteCacheAsync(cacheId);
    }

    public ReconnectingWebSocket subscribe(WebSocket.Listener listener) {
        WebSocket.Listener wrapper = new WebSocket.Listener() {
            @Override
            public void onOpen(WebSocket webSocket) {
                System.out.println("[EmbeddedAeronCache] WebSocket connection opened. Requesting first message...");
                webSocket.request(1);
                listener.onOpen(webSocket);
            }

            @Override
            public CompletionStage<?> onText(WebSocket webSocket, CharSequence data, boolean last) {
                String payload = data.toString();
                System.out.println("[EmbeddedAeronCache] Received raw update message: " + payload);
                try {
                    CacheUpdateEvent event = objectMapper.readValue(payload, CacheUpdateEvent.class);
                    System.out.println("[EmbeddedAeronCache] Parsed update event: " + event.getEventType() + " for key: " + event.getItemKey());
                    updateLocalCache(event);
                } catch (Exception e) {
                    System.err.println("[EmbeddedAeronCache] Error parsing WebSocket update message: " + e.getMessage());
                    e.printStackTrace();
                }
                webSocket.request(1);
                return listener.onText(webSocket, data, last);
            }

            @Override
            public CompletionStage<?> onBinary(WebSocket webSocket, java.nio.ByteBuffer data, boolean last) {
                return listener.onBinary(webSocket, data, last);
            }

            @Override
            public CompletionStage<?> onPing(WebSocket webSocket, java.nio.ByteBuffer message) {
                return listener.onPing(webSocket, message);
            }

            @Override
            public CompletionStage<?> onPong(WebSocket webSocket, java.nio.ByteBuffer message) {
                return listener.onPong(webSocket, message);
            }

            @Override
            public CompletionStage<?> onClose(WebSocket webSocket, int statusCode, String reason) {
                return listener.onClose(webSocket, statusCode, reason);
            }

            @Override
            public void onError(WebSocket webSocket, Throwable error) {
                listener.onError(webSocket, error);
            }
        };
        return client.subscribe(cacheId, wrapper);
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
