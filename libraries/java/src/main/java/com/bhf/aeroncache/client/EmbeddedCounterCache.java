package com.bhf.aeroncache.client;

import com.bhf.aeroncache.models.*;

import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;

public class EmbeddedCounterCache {
    private final AeronCacheClient client;
    private final String cacheId;
    private final Map<String, Long> localCache = new ConcurrentHashMap<>();

    public EmbeddedCounterCache(AeronCacheClient client, String cacheId) {
        this.client = client;
        this.cacheId = cacheId;
    }

    public Long getLocal(String key) {
        return localCache.get(key);
    }

    public Map<String, Long> getLocalCache() {
        return java.util.Collections.unmodifiableMap(localCache);
    }

    public PutItemResponse put(String key, long value) throws Exception {
        return client.putCounter(cacheId, key, value);
    }

    public PutItemResponse putTimed(String key, long value, long ttl) throws Exception {
        return client.putTimedCounter(cacheId, key, value, ttl);
    }

    public CounterResponse get(String key) throws Exception {
        return client.getCounter(cacheId, key);
    }

    public CounterResponse increment(String key, long amount) throws Exception {
        return client.incrementCounter(cacheId, key, amount);
    }

    public CounterResponse decrement(String key, long amount) throws Exception {
        return client.decrementCounter(cacheId, key, amount);
    }

    public CounterResponse set(String key, long value) throws Exception {
        return client.setCounter(cacheId, key, value);
    }

    public DeleteItemResponse remove(String key) throws Exception {
        return client.deleteCounter(cacheId, key);
    }

    public DeleteCacheResponse clear() throws Exception {
        return client.deleteCounterCache(cacheId);
    }

    public CompletableFuture<PutItemResponse> putAsync(String key, long value) {
        return client.putCounterAsync(cacheId, key, value);
    }

    public CompletableFuture<PutItemResponse> putTimedAsync(String key, long value, long ttl) {
        return client.putTimedCounterAsync(cacheId, key, value, ttl);
    }

    public CompletableFuture<CounterResponse> getAsync(String key) {
        return client.getCounterAsync(cacheId, key);
    }

    public CompletableFuture<CounterResponse> incrementAsync(String key, long amount) {
        return client.incrementCounterAsync(cacheId, key, amount);
    }

    public CompletableFuture<CounterResponse> decrementAsync(String key, long amount) {
        return client.decrementCounterAsync(cacheId, key, amount);
    }

    public CompletableFuture<CounterResponse> setAsync(String key, long value) {
        return client.setCounterAsync(cacheId, key, value);
    }

    public CompletableFuture<DeleteItemResponse> removeAsync(String key) {
        return client.deleteCounterAsync(cacheId, key);
    }

    public CompletableFuture<DeleteCacheResponse> clearAsync() {
        return client.deleteCounterCacheAsync(cacheId);
    }

    public ReconnectingWebSocket subscribe(CounterCacheSubscriber subscriber) {
        return subscribe(subscriber, false);
    }

    public ReconnectingWebSocket subscribe(CounterCacheSubscriber subscriber, boolean hydrate) {
        subscriber.setInternalUpdater(this::updateLocalCache);
        return client.subscribeCounter(cacheId, hydrate, subscriber);
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
