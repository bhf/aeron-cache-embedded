package com.bhf.aeroncache.soak;

import com.fasterxml.jackson.databind.ObjectMapper;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.LongAdder;

/**
 * Accumulates per-operation counts and periodic client-heap samples over a soak run, and writes a JSON
 * report at the end. The resolved seed and config are recorded so any finding is reproducible, and the
 * heap samples are the primary leak signal: with a bounded key space, used heap that trends upward with
 * wall-clock time (not with the data set) is a genuine client-side leak.
 */
final class SoakReport {

    private final long seed;
    private final long durationSeconds;
    private final Map<String, Object> config;
    private final long startMillis = System.currentTimeMillis();

    private final Map<String, LongAdder> opCounts = new LinkedHashMap<>();
    private final AtomicLong total = new AtomicLong();
    private final List<long[]> heapSamples = new ArrayList<>(); // [elapsedMillis, usedBytes]

    SoakReport(long seed, long durationSeconds, Map<String, Object> config) {
        this.seed = seed;
        this.durationSeconds = durationSeconds;
        this.config = config;
    }

    void recordCount(String op) {
        opCounts.computeIfAbsent(op, k -> new LongAdder()).increment();
        total.incrementAndGet();
    }

    long total() {
        return total.get();
    }

    void sampleHeap() {
        final Runtime rt = Runtime.getRuntime();
        final long used = rt.totalMemory() - rt.freeMemory();
        heapSamples.add(new long[]{System.currentTimeMillis() - startMillis, used});
    }

    String progressLine() {
        final long used = heapSamples.isEmpty() ? 0 : heapSamples.get(heapSamples.size() - 1)[1];
        return "ops=" + total.get()
                + " elapsed=" + ((System.currentTimeMillis() - startMillis) / 1000) + "s"
                + " heapUsedMB=" + (used / (1024 * 1024));
    }

    void writeTo(Path path) throws IOException {
        sampleHeap();
        final Map<String, Object> root = new LinkedHashMap<>();
        root.put("seed", seed);
        root.put("durationSeconds", durationSeconds);
        root.put("config", config);
        root.put("startMillis", startMillis);
        root.put("endMillis", System.currentTimeMillis());
        root.put("totalOps", total.get());
        final Map<String, Long> counts = new LinkedHashMap<>();
        opCounts.forEach((k, v) -> counts.put(k, v.sum()));
        root.put("opCounts", counts);
        final List<Map<String, Long>> samples = new ArrayList<>();
        for (long[] s : heapSamples) {
            final Map<String, Long> m = new LinkedHashMap<>();
            m.put("elapsedMillis", s[0]);
            m.put("usedHeapBytes", s[1]);
            samples.add(m);
        }
        root.put("heapSamples", samples);

        Files.createDirectories(path.getParent());
        new ObjectMapper().writerWithDefaultPrettyPrinter().writeValue(path.toFile(), root);
    }
}
