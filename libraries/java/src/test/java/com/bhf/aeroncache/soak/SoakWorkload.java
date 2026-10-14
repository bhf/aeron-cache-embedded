package com.bhf.aeroncache.soak;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Random;
import java.util.concurrent.TimeUnit;
import java.util.function.Supplier;

/**
 * The single-writer, time-bounded soak workload.
 *
 * <p>One thread drives a weighted, seeded-random mix of key-value and counter operations through a
 * {@link SoakClient} against a bounded key space spread over a handful of caches, keeping a
 * {@link SoakOracle} model in lock-step with every applied mutation. Each operation is verified
 * immediately where the backend echoes a value (gets, counter inc/dec/set); the full model is reconciled
 * against the backend every {@link SoakConfig#verifyEvery} operations and once more at the end. Any
 * divergence, non-{@code SUCCESS} status, thrown error, or operation exceeding
 * {@link SoakConfig#opTimeoutSeconds} fails the run with the seed logged for reproduction.
 */
final class SoakWorkload {

    enum OpType {
        KV_PUT, KV_GET, KV_REMOVE, KV_CLEAR, KV_DELETE_RECREATE,
        COUNTER_PUT, COUNTER_INC, COUNTER_DEC, COUNTER_SET, COUNTER_GET
    }

    private static final char[] ALPHABET =
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".toCharArray();
    private static final long PROGRESS_EVERY = 5_000L;
    private static final String SUCCESS = "SUCCESS";

    private final SoakClient client;
    private final SoakConfig cfg;
    private final SoakReport report;
    private final SoakOracle oracle = new SoakOracle();
    private final Random rnd;
    private final long opTimeoutNanos;
    private final OpType[] opTable;

    private final List<String> kvCacheIds = new ArrayList<>();
    private final List<String> counterCacheIds = new ArrayList<>();

    SoakWorkload(SoakClient client, SoakConfig cfg) {
        this.client = client;
        this.cfg = cfg;
        this.report = new SoakReport(cfg.seed, cfg.durationSeconds, cfg.toConfigMap());
        this.rnd = new Random(cfg.seed);
        this.opTimeoutNanos = TimeUnit.SECONDS.toNanos(cfg.opTimeoutSeconds);
        this.opTable = buildOpTable(cfg);
    }

    SoakReport report() {
        return report;
    }

    void run() throws Exception {
        System.out.println("[soak] starting " + cfg);
        try {
            setupCaches();
            final long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(cfg.durationSeconds);
            while (System.nanoTime() < deadline) {
                final OpType op = opTable[rnd.nextInt(opTable.length)];
                final OpType applied = execute(op);
                report.recordCount(applied.name());

                final long ops = report.total();
                if (ops % cfg.verifyEvery == 0) {
                    reconcileAll();
                }
                if (ops % PROGRESS_EVERY == 0) {
                    report.sampleHeap();
                    System.out.println("[soak] " + report.progressLine());
                }
            }
            reconcileAll(); // end on a verified-consistent state
            System.out.println("[soak] complete: " + report.progressLine());
        } catch (Throwable t) {
            // Make every failure reproducible: the seed replays the exact op sequence.
            throw new AssertionError("Soak failed (seed=" + cfg.seed + ", transport=" + cfg.transport
                    + ", opsSoFar=" + report.total() + "): " + t.getMessage(), t);
        }
    }

    // ---------------------------------------------------------------- setup

    private void setupCaches() throws Exception {
        for (int i = 0; i < cfg.kvCacheCount; i++) {
            final String id = "soak-" + cfg.transport + "-kv-" + i;
            client.createKvCache(id);
            oracle.registerKvCache(id);
            kvCacheIds.add(id);
        }
        for (int i = 0; i < cfg.counterCacheCount; i++) {
            final String id = "soak-" + cfg.transport + "-ctr-" + i;
            client.createCounterCache(id);
            oracle.registerCounterCache(id);
            counterCacheIds.add(id);
        }
    }

    // ---------------------------------------------------------------- ops

    /** @return the op actually applied (an op may fall back when its precondition is unmet). */
    private OpType execute(OpType op) throws Exception {
        switch (op) {
            case KV_PUT: {
                final String cache = randomKv(), key = randomKey(), val = randomValue();
                check(SUCCESS.equals(timed(() -> client.put(cache, key, val))), () -> "put !SUCCESS " + cache + "/" + key);
                oracle.putKv(cache, key, val);
                return OpType.KV_PUT;
            }
            case KV_GET: {
                final String cache = randomKv(), key = randomKey();
                final String actual = timed(() -> client.get(cache, key));
                final String expected = oracle.getKv(cache, key);
                check(Objects.equals(actual, expected),
                        () -> "kv get mismatch " + cache + "/" + key + " expected=" + expected + " actual=" + actual);
                return OpType.KV_GET;
            }
            case KV_REMOVE: {
                final String cache = randomKv();
                final List<String> keys = oracle.kvKeys(cache);
                if (keys.isEmpty()) {
                    return execute(OpType.KV_PUT);
                }
                final String key = keys.get(rnd.nextInt(keys.size()));
                check(SUCCESS.equals(timed(() -> client.remove(cache, key))), () -> "remove !SUCCESS " + cache + "/" + key);
                oracle.removeKv(cache, key);
                return OpType.KV_REMOVE;
            }
            case KV_CLEAR: {
                final String cache = randomKv();
                check(SUCCESS.equals(timed(() -> client.clear(cache))), () -> "clear !SUCCESS " + cache);
                oracle.clearKv(cache);
                return OpType.KV_CLEAR;
            }
            case KV_DELETE_RECREATE: {
                final String cache = randomKv();
                check(SUCCESS.equals(timed(() -> client.deleteCache(cache))), () -> "deleteCache !SUCCESS " + cache);
                timedVoid(() -> client.createKvCache(cache));
                oracle.clearKv(cache);
                return OpType.KV_DELETE_RECREATE;
            }
            case COUNTER_PUT: {
                // putCounter is the counter creator (set/inc/dec require the key to already exist).
                final String cache = randomCounter(), key = randomKey();
                final long value = rnd.nextInt(1_000);
                check(SUCCESS.equals(timed(() -> client.putCounter(cache, key, value))),
                        () -> "putCounter !SUCCESS " + cache + "/" + key);
                oracle.setCounter(cache, key, value);
                verifyCounter(cache, key, value);
                return OpType.COUNTER_PUT;
            }
            case COUNTER_INC:
            case COUNTER_DEC: {
                final String cache = randomCounter();
                final String key = pickPresentCounterKey(cache);
                if (key == null) {
                    return execute(OpType.COUNTER_PUT); // seed a counter first
                }
                final long amount = 1 + rnd.nextInt(10);
                final long delta = op == OpType.COUNTER_INC ? amount : -amount;
                timed(() -> op == OpType.COUNTER_INC
                        ? client.incrementCounter(cache, key, amount)
                        : client.decrementCounter(cache, key, amount));
                final long expected = oracle.addToCounter(cache, key, delta);
                // Verify via read-back rather than the mutation's echoed value (which varies by transport).
                verifyCounter(cache, key, expected);
                return op;
            }
            case COUNTER_SET: {
                final String cache = randomCounter();
                final String key = pickPresentCounterKey(cache);
                if (key == null) {
                    return execute(OpType.COUNTER_PUT);
                }
                final long value = rnd.nextInt(1_000);
                timed(() -> client.setCounter(cache, key, value));
                oracle.setCounter(cache, key, value);
                verifyCounter(cache, key, value);
                return OpType.COUNTER_SET;
            }
            case COUNTER_GET: {
                final String cache = randomCounter();
                final String key = pickPresentCounterKey(cache);
                if (key == null) {
                    return execute(OpType.COUNTER_PUT);
                }
                verifyCounter(cache, key, oracle.getCounter(cache, key));
                return OpType.COUNTER_GET;
            }
            default:
                throw new IllegalStateException("unhandled op " + op);
        }
    }

    // ---------------------------------------------------------------- reconciliation

    private void reconcileAll() throws Exception {
        for (String cache : kvCacheIds) {
            final Map<String, String> expected = oracle.kvSnapshot(cache);
            final Map<String, String> actual = client.listKv(cache);
            check(actual.equals(expected),
                    () -> "kv reconcile mismatch " + cache + " expectedSize=" + expected.size()
                            + " actualSize=" + actual.size() + " diff=" + firstDiff(expected, actual));
        }
        // The gateway client exposes no counter-listing op, so counters reconcile per key over the
        // bounded key space (inline inc/dec/set verification already covers value drift continuously).
        for (String cache : counterCacheIds) {
            for (String key : oracle.counterKeys(cache)) {
                final Long expected = oracle.getCounter(cache, key);
                final Long actual = client.getCounter(cache, key);
                check(Objects.equals(actual, expected),
                        () -> "counter reconcile mismatch " + cache + "/" + key + " expected=" + expected + " actual=" + actual);
            }
        }
    }

    // ---------------------------------------------------------------- helpers

    private String randomKv() {
        return kvCacheIds.get(rnd.nextInt(kvCacheIds.size()));
    }

    private String randomCounter() {
        return counterCacheIds.get(rnd.nextInt(counterCacheIds.size()));
    }

    private String randomKey() {
        return "k" + rnd.nextInt(cfg.keySpace);
    }

    private void verifyCounter(String cache, String key, Long expected) throws Exception {
        final Long actual = timed(() -> client.getCounter(cache, key));
        check(Objects.equals(actual, expected),
                () -> "counter mismatch " + cache + "/" + key + " expected=" + expected + " actual=" + actual);
    }

    private String pickPresentCounterKey(String cache) {
        final List<String> keys = oracle.counterKeys(cache);
        return keys.isEmpty() ? null : keys.get(rnd.nextInt(keys.size()));
    }

    private String randomValue() {
        final char[] c = new char[cfg.valueSizeBytes];
        for (int i = 0; i < c.length; i++) {
            c[i] = ALPHABET[rnd.nextInt(ALPHABET.length)];
        }
        return new String(c);
    }

    private <T> T timed(ThrowingSupplier<T> op) throws Exception {
        final long start = System.nanoTime();
        final T result = op.get();
        final long elapsed = System.nanoTime() - start;
        if (elapsed > opTimeoutNanos) {
            throw new AssertionError("operation exceeded " + cfg.opTimeoutSeconds + "s ("
                    + TimeUnit.NANOSECONDS.toMillis(elapsed) + "ms)");
        }
        return result;
    }

    private void timedVoid(ThrowingRunnable op) throws Exception {
        timed(() -> {
            op.run();
            return null;
        });
    }

    private static void check(boolean condition, Supplier<String> message) {
        if (!condition) {
            throw new AssertionError(message.get());
        }
    }

    private static String firstDiff(Map<String, String> expected, Map<String, String> actual) {
        for (var e : expected.entrySet()) {
            if (!Objects.equals(actual.get(e.getKey()), e.getValue())) {
                return "key=" + e.getKey() + " expected=" + e.getValue() + " actual=" + actual.get(e.getKey());
            }
        }
        for (String k : actual.keySet()) {
            if (!expected.containsKey(k)) {
                return "spurious key=" + k + " actual=" + actual.get(k);
            }
        }
        return "(none)";
    }

    private OpType[] buildOpTable(SoakConfig cfg) {
        final List<OpType> table = new ArrayList<>();
        if (cfg.kvCacheCount > 0) {
            addWeighted(table, OpType.KV_PUT, 5);
            addWeighted(table, OpType.KV_GET, 5);
            addWeighted(table, OpType.KV_REMOVE, 2);
            addWeighted(table, OpType.KV_CLEAR, 1);
            addWeighted(table, OpType.KV_DELETE_RECREATE, 1);
        }
        if (cfg.counterCacheCount > 0) {
            addWeighted(table, OpType.COUNTER_PUT, 3);
            addWeighted(table, OpType.COUNTER_INC, 4);
            addWeighted(table, OpType.COUNTER_DEC, 2);
            addWeighted(table, OpType.COUNTER_SET, 2);
            addWeighted(table, OpType.COUNTER_GET, 4);
        }
        if (table.isEmpty()) {
            throw new IllegalArgumentException("soak needs at least one kv or counter cache");
        }
        return table.toArray(new OpType[0]);
    }

    private static void addWeighted(List<OpType> table, OpType op, int weight) {
        for (int i = 0; i < weight; i++) {
            table.add(op);
        }
    }

    @FunctionalInterface
    private interface ThrowingSupplier<T> {
        T get() throws Exception;
    }

    @FunctionalInterface
    private interface ThrowingRunnable {
        void run() throws Exception;
    }
}
