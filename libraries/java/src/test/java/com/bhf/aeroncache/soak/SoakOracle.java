package com.bhf.aeroncache.soak;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * The in-memory model the workload keeps in lock-step with the backend: the expected contents of every
 * key-value and counter cache. Reconciliation compares this model against what the client reads back; any
 * divergence is a soak finding. The model is bounded by the configured key space, so it does not itself
 * grow with time.
 */
final class SoakOracle {

    private final Map<String, Map<String, String>> kv = new LinkedHashMap<>();
    private final Map<String, Map<String, Long>> counters = new LinkedHashMap<>();

    void registerKvCache(String cacheId) {
        kv.put(cacheId, new LinkedHashMap<>());
    }

    void registerCounterCache(String cacheId) {
        counters.put(cacheId, new LinkedHashMap<>());
    }

    // ---------------------------------------------------------------- key-value

    void putKv(String cacheId, String key, String value) {
        kv.get(cacheId).put(key, value);
    }

    void removeKv(String cacheId, String key) {
        kv.get(cacheId).remove(key);
    }

    void clearKv(String cacheId) {
        kv.get(cacheId).clear();
    }

    String getKv(String cacheId, String key) {
        return kv.get(cacheId).get(key);
    }

    /** Keys currently present in a kv cache (a copy, safe to pick from while mutating). */
    List<String> kvKeys(String cacheId) {
        return new ArrayList<>(kv.get(cacheId).keySet());
    }

    Map<String, String> kvSnapshot(String cacheId) {
        return kv.get(cacheId);
    }

    // ---------------------------------------------------------------- counters

    void setCounter(String cacheId, String key, long value) {
        counters.get(cacheId).put(key, value);
    }

    long addToCounter(String cacheId, String key, long delta) {
        final long next = counters.get(cacheId).getOrDefault(key, 0L) + delta;
        counters.get(cacheId).put(key, next);
        return next;
    }

    Long getCounter(String cacheId, String key) {
        return counters.get(cacheId).get(key);
    }

    List<String> counterKeys(String cacheId) {
        return new ArrayList<>(counters.get(cacheId).keySet());
    }
}
