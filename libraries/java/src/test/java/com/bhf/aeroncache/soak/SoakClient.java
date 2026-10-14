package com.bhf.aeroncache.soak;

import java.util.Map;

/**
 * Transport-neutral facade over the client libraries, so a single {@link SoakWorkload} can drive the HTTP,
 * bidi-WebSocket or Aeron-gateway client. All three expose identical synchronous op methods returning the
 * same model types, so each implementation is a thin wrapper (see {@link HttpSoakClient},
 * {@link BidiSoakClient}, {@link GatewaySoakClient}).
 *
 * <p>Methods return the operation's {@code operationStatus} string (e.g. {@code "SUCCESS"}) where the
 * workload asserts on it, the read-back value for reads, or {@code null} for an absent key.
 */
interface SoakClient extends AutoCloseable {

    /** Transport label, used in the report and cache-id namespacing. */
    String name();

    void createKvCache(String cacheId) throws Exception;

    void createCounterCache(String cacheId) throws Exception;

    /** @return operationStatus. */
    String put(String cacheId, String key, String value) throws Exception;

    /** @return the stored value, or {@code null} if the key is absent. */
    String get(String cacheId, String key) throws Exception;

    /** @return operationStatus. */
    String remove(String cacheId, String key) throws Exception;

    /** @return operationStatus. */
    String clear(String cacheId) throws Exception;

    /** @return operationStatus. */
    String deleteCache(String cacheId) throws Exception;

    /** List the full contents of a key-value cache (used for bidirectional reconciliation). */
    Map<String, String> listKv(String cacheId) throws Exception;

    /** @return operationStatus. */
    String putCounter(String cacheId, String key, long value) throws Exception;

    /** @return the resulting counter value. */
    long incrementCounter(String cacheId, String key, long amount) throws Exception;

    /** @return the resulting counter value. */
    long decrementCounter(String cacheId, String key, long amount) throws Exception;

    /** @return the resulting counter value. */
    long setCounter(String cacheId, String key, long value) throws Exception;

    /** @return the counter value, or {@code null} if the key is absent. */
    Long getCounter(String cacheId, String key) throws Exception;

    @Override
    void close() throws Exception;
}
