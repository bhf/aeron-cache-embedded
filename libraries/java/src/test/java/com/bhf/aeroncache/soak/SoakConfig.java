package com.bhf.aeroncache.soak;

import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Configuration for a client soak run, resolved from {@code -Dsoak.*} system properties (forwarded from
 * {@code -Psoak.*} by the {@code soakTest} Gradle task) with sensible defaults. The resolved values —
 * crucially the {@link #seed} — are recorded in the report so any failure is reproducible by re-running
 * with the same {@code -Psoak.seed}.
 *
 * <p>The run is purely time-bounded: the workload drives a weighted, seeded-random mix of operations until
 * {@link #durationSeconds} of wall-clock has elapsed. Because the key space is bounded, the data held by
 * both the backend and the oracle is bounded too, so client heap that grows with <em>time</em> rather than
 * with the key space is a genuine leak — which is the point of the soak.
 */
final class SoakConfig {

    /** Which client transport to drive: {@code http}, {@code bidi} or {@code gateway}. */
    final String transport;
    /** Total wall-clock duration to drive load for. */
    final long durationSeconds;
    /** PRNG seed; every random choice derives from it, so a run is reproducible given the seed. */
    final long seed;
    /** Number of key-value caches the workload spreads load across. */
    final int kvCacheCount;
    /** Number of counter caches the workload spreads load across. */
    final int counterCacheCount;
    /** Size of the key space per cache; a small space forces overwrites, removes and collisions. */
    final int keySpace;
    /** Size in bytes of randomly generated values. */
    final int valueSizeBytes;
    /** Run a full oracle reconciliation sweep every N operations. */
    final int verifyEvery;
    /** Per-operation timeout; a breach means the backend/client stalled — a genuine soak finding. */
    final int opTimeoutSeconds;

    private SoakConfig(String transport, long durationSeconds, long seed, int kvCacheCount,
                       int counterCacheCount, int keySpace, int valueSizeBytes, int verifyEvery,
                       int opTimeoutSeconds) {
        this.transport = transport;
        this.durationSeconds = durationSeconds;
        this.seed = seed;
        this.kvCacheCount = kvCacheCount;
        this.counterCacheCount = counterCacheCount;
        this.keySpace = keySpace;
        this.valueSizeBytes = valueSizeBytes;
        this.verifyEvery = verifyEvery;
        this.opTimeoutSeconds = opTimeoutSeconds;
    }

    static SoakConfig fromSystemProperties() {
        return new SoakConfig(
                strProp("soak.transport", "http"),
                longProp("soak.durationSeconds", 1800L),
                seedProp("soak.seed"),
                intProp("soak.kvCacheCount", 4),
                intProp("soak.counterCacheCount", 4),
                intProp("soak.keySpace", 500),
                intProp("soak.valueSizeBytes", 160),
                intProp("soak.verifyEvery", 2000),
                intProp("soak.opTimeoutSeconds", 30));
    }

    /** Ordered config fields for the run report (seed and duration are recorded separately). */
    Map<String, Object> toConfigMap() {
        var map = new LinkedHashMap<String, Object>();
        map.put("transport", transport);
        map.put("kvCacheCount", kvCacheCount);
        map.put("counterCacheCount", counterCacheCount);
        map.put("keySpace", keySpace);
        map.put("valueSizeBytes", valueSizeBytes);
        map.put("verifyEvery", verifyEvery);
        map.put("opTimeoutSeconds", opTimeoutSeconds);
        return map;
    }

    private static String strProp(String key, String def) {
        final String v = System.getProperty(key);
        return v == null || v.isBlank() ? def : v.trim();
    }

    private static long longProp(String key, long def) {
        final String v = System.getProperty(key);
        return v == null || v.isBlank() ? def : Long.parseLong(v.trim());
    }

    private static int intProp(String key, int def) {
        final String v = System.getProperty(key);
        return v == null || v.isBlank() ? def : Integer.parseInt(v.trim());
    }

    /** A blank or missing seed yields a fresh random seed (recorded in the report for reproduction). */
    private static long seedProp(String key) {
        final String v = System.getProperty(key);
        return v == null || v.isBlank() ? new java.util.Random().nextLong() : Long.parseLong(v.trim());
    }

    @Override
    public String toString() {
        return "SoakConfig{transport=" + transport
                + ", durationSeconds=" + durationSeconds
                + ", seed=" + seed
                + ", kvCacheCount=" + kvCacheCount
                + ", counterCacheCount=" + counterCacheCount
                + ", keySpace=" + keySpace
                + ", valueSizeBytes=" + valueSizeBytes
                + ", verifyEvery=" + verifyEvery
                + ", opTimeoutSeconds=" + opTimeoutSeconds + '}';
    }
}
