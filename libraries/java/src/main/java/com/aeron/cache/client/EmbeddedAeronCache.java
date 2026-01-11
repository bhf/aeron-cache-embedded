package com.aeron.cache.client;

import java.net.http.WebSocket;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionStage;
import java.util.concurrent.ConcurrentHashMap;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class EmbeddedAeronCache {
    private final AeronCacheClient client;
    private final String cacheId;
    private final Map<String, String> localCache = new ConcurrentHashMap<>();

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

    public String put(String key, String value) throws Exception {
        return client.putItem(cacheId, key, value);
    }

    public String get(String key) throws Exception {
        return client.getItem(cacheId, key);
    }

    public String remove(String key) throws Exception {
        return client.deleteItem(cacheId, key);
    }

    public String clear() throws Exception {
        return client.deleteCache(cacheId);
    }

    public CompletableFuture<String> putAsync(String key, String value) {
        return client.putItemAsync(cacheId, key, value);
    }

    public CompletableFuture<String> getAsync(String key) {
        return client.getItemAsync(cacheId, key);
    }

    public CompletableFuture<String> removeAsync(String key) {
        return client.deleteItemAsync(cacheId, key);
    }

    public CompletableFuture<String> clearAsync() {
        return client.deleteCacheAsync(cacheId);
    }

    public CompletableFuture<WebSocket> subscribe(WebSocket.Listener listener) {
        WebSocket.Listener wrapper = new WebSocket.Listener() {
            @Override
            public void onOpen(WebSocket webSocket) {
                listener.onOpen(webSocket);
            }

            @Override
            public CompletionStage<?> onText(WebSocket webSocket, CharSequence data, boolean last) {
                updateLocalCache(data.toString());
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
            public void onClose(WebSocket webSocket, int statusCode, String reason) {
                listener.onClose(webSocket, statusCode, reason);
            }

            @Override
            public void onError(WebSocket webSocket, Throwable error) {
                listener.onError(webSocket, error);
            }
        };
        return client.subscribe(cacheId, wrapper);
    }

    private void updateLocalCache(String json) {
        String eventType = extractJsonValue(json, "eventType");
        if (eventType == null) return;

        switch (eventType) {
            case "ADD_ITEM":
                String key = extractJsonValue(json, "itemKey");
                String value = extractJsonValue(json, "itemValue");
                if (key != null && value != null) {
                    localCache.put(key, value);
                }
                break;
            case "REMOVE_ITEM":
                String rmKey = extractJsonValue(json, "itemKey");
                if (rmKey != null) {
                    localCache.remove(rmKey);
                }
                break;
            case "DELETE_CACHE":
            case "CLEAR_CACHE":
                localCache.clear();
                break;
        }
    }

    private String extractJsonValue(String json, String key) {
        Pattern pattern = Pattern.compile("\"" + key + "\"\\s*:\\s*\"([^\"]+)\"");
        Matcher matcher = pattern.matcher(json);
        if (matcher.find()) {
            return matcher.group(1);
        }
        return null;
    }
}
