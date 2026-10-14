package com.bhf.aeroncache.soak;

import com.bhf.aeroncache.client.bidi.AeronBidiClient;
import com.bhf.aeroncache.models.CacheItem;
import com.bhf.aeroncache.models.CounterResponse;
import com.bhf.aeroncache.models.GetItemResponse;

import java.util.LinkedHashMap;
import java.util.Map;

/** {@link SoakClient} over the bidirectional-WebSocket {@link AeronBidiClient}. */
final class BidiSoakClient implements SoakClient {

    private static final String SUCCESS = "SUCCESS";

    private final AeronBidiClient client;

    BidiSoakClient(String wsUrl) {
        this.client = new AeronBidiClient(wsUrl);
        client.connect();
    }

    @Override
    public String name() {
        return "bidi";
    }

    @Override
    public void createKvCache(String cacheId) throws Exception {
        client.createCache(cacheId);
    }

    @Override
    public void createCounterCache(String cacheId) throws Exception {
        client.createCounterCache(cacheId);
    }

    @Override
    public String put(String cacheId, String key, String value) throws Exception {
        return client.putItem(cacheId, key, value).getOperationStatus();
    }

    @Override
    public String get(String cacheId, String key) throws Exception {
        final GetItemResponse r = client.getItem(cacheId, key);
        return SUCCESS.equals(r.getOperationStatus()) ? r.getValue() : null;
    }

    @Override
    public String remove(String cacheId, String key) throws Exception {
        return client.deleteItem(cacheId, key).getOperationStatus();
    }

    @Override
    public String clear(String cacheId) throws Exception {
        return client.clearCache(cacheId).getOperationStatus();
    }

    @Override
    public String deleteCache(String cacheId) throws Exception {
        return client.deleteCache(cacheId).getOperationStatus();
    }

    @Override
    public Map<String, String> listKv(String cacheId) throws Exception {
        final Map<String, String> out = new LinkedHashMap<>();
        for (CacheItem item : client.getCacheItems(cacheId).getItems()) {
            out.put(item.getKey(), item.getValue());
        }
        return out;
    }

    @Override
    public String putCounter(String cacheId, String key, long value) throws Exception {
        return client.putCounter(cacheId, key, value).getOperationStatus();
    }

    @Override
    public long incrementCounter(String cacheId, String key, long amount) throws Exception {
        return client.incrementCounter(cacheId, key, amount).getValue();
    }

    @Override
    public long decrementCounter(String cacheId, String key, long amount) throws Exception {
        return client.decrementCounter(cacheId, key, amount).getValue();
    }

    @Override
    public long setCounter(String cacheId, String key, long value) throws Exception {
        return client.setCounter(cacheId, key, value).getValue();
    }

    @Override
    public Long getCounter(String cacheId, String key) throws Exception {
        final CounterResponse r = client.getCounter(cacheId, key);
        return SUCCESS.equals(r.getOperationStatus()) ? r.getValue() : null;
    }

    @Override
    public void close() {
        client.close();
    }
}
