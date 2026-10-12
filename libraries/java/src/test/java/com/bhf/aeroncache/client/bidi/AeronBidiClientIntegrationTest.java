package com.bhf.aeroncache.client.bidi;

import com.bhf.aeroncache.models.BulkCacheOpsRequest;
import com.bhf.aeroncache.models.BulkCacheOpsResponse;
import com.bhf.aeroncache.models.BulkOperationType;
import com.bhf.aeroncache.models.CacheItem;
import com.bhf.aeroncache.models.CacheOperationRequest;
import com.bhf.aeroncache.models.CacheOperationResponse;
import com.bhf.aeroncache.models.CacheUpdateEvent;
import com.bhf.aeroncache.models.CounterItem;
import com.bhf.aeroncache.models.CounterUpdateEvent;
import com.bhf.aeroncache.models.GetCacheResponse;
import com.bhf.aeroncache.models.GetCountersResponse;
import com.bhf.aeroncache.models.StatEntry;
import com.bhf.aeroncache.models.TimerInfo;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

import java.util.List;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.stream.Collectors;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * End-to-end tests for {@link AeronBidiClient} against a live backend exposing the bidirectional
 * WebSocket transport ({@code /api/ws/v1/bidi}).
 * <p>
 * Enabled only when {@code AERON_CACHE_BASE_URL} is set (mirroring the HTTP integration test's
 * convention). The WebSocket URL is taken from {@code AERON_CACHE_WS_URL}, or derived from the base URL
 * by swapping the {@code http(s)} scheme for {@code ws(s)}.
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
class AeronBidiClientIntegrationTest {

    /** Entry/timer count chosen to overflow a single server batch frame (matches the gateway batch tests). */
    private static final int ENTRY_COUNT = 1_000;
    /**
     * Per-bulk chunk size kept well under the bidi WebSocket ~64KB max text-message limit
     * ({@code BULK_CHUNK * BATCH_VALUE} length must stay below it, or the server closes with 1009).
     */
    private static final int BULK_CHUNK = 100;
    /** Value long enough that {@value #ENTRY_COUNT} entries comfortably exceed one batch frame. */
    private static final String BATCH_VALUE = "v".repeat(300);

    private AeronBidiClient client;

    @BeforeAll
    void setUp() {
        final String baseUrl = System.getenv("AERON_CACHE_BASE_URL");
        Assumptions.assumeTrue(baseUrl != null && !baseUrl.isEmpty(),
                "Integration tests disabled: AERON_CACHE_BASE_URL not set");

        String wsUrl = System.getenv("AERON_CACHE_WS_URL");
        if (wsUrl == null || wsUrl.isEmpty()) {
            wsUrl = baseUrl.replace("http://", "ws://").replace("https://", "wss://");
        }
        client = new AeronBidiClient(wsUrl);
        client.connect();
    }

    @AfterAll
    void tearDown() {
        if (client != null) {
            client.close();
        }
    }

    @Test
    void cacheLifecycle() throws Exception {
        final String cacheId = "bidi-cache-" + UUID.randomUUID().toString().substring(0, 8);
        assertEquals(cacheId, client.createCache(cacheId).getCacheId());
        assertEquals("SUCCESS", client.putItem(cacheId, "k1", "v1").getOperationStatus());
        assertEquals("v1", client.getItem(cacheId, "k1").getValue());

        client.putItem(cacheId, "doc", "{\"a\":1}");
        client.patchItem(cacheId, "doc", "{\"b\":2}");
        String doc = "";
        for (int i = 0; i < 25; i++) {
            doc = client.getItem(cacheId, "doc").getValue();
            if (doc != null && doc.contains("\"a\":1") && doc.contains("\"b\":2")) {
                break;
            }
            Thread.sleep(200);
        }
        assertTrue(doc != null && doc.contains("\"a\":1"), "patched doc should retain original field: " + doc);
        assertTrue(doc.contains("\"b\":2"), "patched doc should contain merged field: " + doc);

        final GetCacheResponse items = client.getCacheItems(cacheId);
        final Set<String> keys = items.getItems().stream().map(CacheItem::getKey).collect(Collectors.toSet());
        assertEquals(Set.of("k1", "doc"), keys);

        assertEquals("SUCCESS", client.deleteItem(cacheId, "k1").getOperationStatus());
        assertEquals("SUCCESS", client.clearCache(cacheId).getOperationStatus());
        client.deleteCache(cacheId);
    }

    @Test
    void counterLifecycle() throws Exception {
        final String cacheId = "bidi-counter-" + UUID.randomUUID().toString().substring(0, 8);
        client.createCounterCache(cacheId);
        client.putCounter(cacheId, "hits", 10);
        assertEquals(15L, client.incrementCounter(cacheId, "hits", 5).getValue());
        assertEquals(12L, client.decrementCounter(cacheId, "hits", 3).getValue());
        assertEquals(100L, client.setCounter(cacheId, "hits", 100).getValue());
        assertEquals(100L, client.getCounter(cacheId, "hits").getValue());

        client.putCounter(cacheId, "misses", 7);
        final GetCountersResponse got = client.getCounterItems(cacheId);
        final java.util.Map<String, Long> byKey = got.getItems().stream()
                .collect(Collectors.toMap(CounterItem::getKey, CounterItem::getValue));
        assertEquals(java.util.Map.of("hits", 100L, "misses", 7L), byKey);

        client.clearCounterCache(cacheId);
        client.deleteCounterCache(cacheId);
    }

    @Test
    void getStats() throws Exception {
        final String cacheId = "bidi-stats-" + UUID.randomUUID().toString().substring(0, 8);
        client.createCache(cacheId);
        client.putItem(cacheId, "k", "v");
        final List<StatEntry> stats = client.getStats();
        assertNotNull(stats);
        client.deleteCache(cacheId);
    }

    @Test
    void getTimers() throws Exception {
        final String cacheId = "bidi-timers-" + UUID.randomUUID().toString().substring(0, 8);
        client.createCache(cacheId);
        // A timed entry schedules a pending TTL removal timer.
        client.putTimedItem(cacheId, "ttl-key", "v", 600_000L);

        final List<TimerInfo> timers = client.getTimers();
        final TimerInfo timer = timers.stream()
                .filter(t -> cacheId.equals(t.getCacheId()) && "ttl-key".equals(t.getKey()))
                .findFirst().orElse(null);
        assertNotNull(timer, "expected a pending timer for " + cacheId + "/ttl-key");
        assertEquals("CACHE", timer.getTimerType());
        assertTrue(timer.getDeadline() > 0, "timer deadline should be a positive epoch millis");
        client.deleteCache(cacheId);
    }

    @Test
    void bulkOps() throws Exception {
        final String cacheId = "bidi-bulk-" + UUID.randomUUID().toString().substring(0, 8);
        client.createCache(cacheId);

        final BulkCacheOpsRequest request = BulkCacheOpsRequest.builder()
                .requestId(UUID.randomUUID().toString())
                .addOperation(CacheOperationRequest.builder()
                        .operationType(BulkOperationType.ADD_ITEM)
                        .requestId("op-1").cacheId(cacheId).key("bk1").value("bv1").build())
                .addOperation(CacheOperationRequest.builder()
                        .operationType(BulkOperationType.ADD_ITEM)
                        .requestId("op-2").cacheId(cacheId).key("bk2").value("bv2").build())
                .addOperation(CacheOperationRequest.builder()
                        .operationType(BulkOperationType.GET_ITEM)
                        .requestId("op-3").cacheId(cacheId).key("bk1").build())
                .build();

        final BulkCacheOpsResponse response = client.bulkOps(request);
        assertEquals(request.getRequestId(), response.getRequestId());
        assertNotNull(response.getOperationResponses());
        assertEquals(3, response.getOperationResponses().size());

        final CacheOperationResponse getResult = response.getOperationResponses().stream()
                .filter(r -> "op-3".equals(r.getRequestId()))
                .findFirst().orElseThrow();
        assertEquals("bv1", getResult.getValue());

        assertEquals("bv2", client.getItem(cacheId, "bk2").getValue());
        client.deleteCache(cacheId);
    }

    @Test
    void cancelItemRemoval() throws Exception {
        final String cacheId = "bidi-cancel-" + UUID.randomUUID().toString().substring(0, 8);
        client.createCache(cacheId);
        client.putTimedItem(cacheId, "keep", "val", 2000);
        assertEquals("keep", client.cancelItemRemoval(cacheId, "keep").getKey());
        Thread.sleep(3000);
        assertEquals("val", client.getItem(cacheId, "keep").getValue(),
                "item should survive past its TTL after cancelling removal");
        client.deleteCache(cacheId);
    }

    /**
     * Multi-batch reassembly over the bidi transport, entries variant.
     * <p>
     * COMPLETENESS-LEVEL (not flag-level): {@link AeronBidiClient} accumulates the server's
     * {@code entries} frames internally via its {@code EntriesAccumulator}, consuming each frame's
     * {@code endOfBatch} flag inside its message handler and completing the future only on the final
     * frame. The flag is therefore not observable through the public API (unlike the low-level
     * {@code GatewayClient}, which exposes per-batch callbacks). So rather than asserting the flag
     * sequence, this populates enough entries to force a server-side split and asserts that <em>all</em>
     * {@value #ENTRY_COUNT} entries come back intact and correct — which can only hold if every batch
     * frame was reassembled.
     */
    @Test
    void getCacheItemsReassemblesAcrossMultipleBatches() throws Exception {
        final String cacheId = "bidi-batch-entries-" + UUID.randomUUID().toString().substring(0, 8);
        client.createCache(cacheId);
        populateEntries(cacheId, ENTRY_COUNT, 0L);

        final GetCacheResponse items = client.getCacheItems(cacheId);
        final java.util.Map<String, String> byKey = items.getItems().stream()
                .collect(Collectors.toMap(CacheItem::getKey, CacheItem::getValue));
        assertEquals(ENTRY_COUNT, byKey.size(),
                "every entry should be reassembled from the batched response");
        for (int i = 0; i < ENTRY_COUNT; i++) {
            assertEquals(BATCH_VALUE, byKey.get("k" + i), "missing/incorrect value for k" + i);
        }
        client.deleteCache(cacheId);
    }

    /**
     * Multi-batch reassembly over the bidi transport, timers variant. See
     * {@link #getCacheItemsReassemblesAcrossMultipleBatches()} for why this is completeness-level:
     * {@code getTimers} likewise accumulates {@code timers} frames internally behind its future, so the
     * {@code endOfBatch} flag is not observable. Each timed entry schedules a pending TTL removal timer;
     * populating {@value #ENTRY_COUNT} of them forces a server-side split, and asserting all of our
     * cache's timers survive proves reassembly.
     */
    @Test
    void getTimersReassemblesAcrossMultipleBatches() throws Exception {
        final String cacheId = "bidi-batch-timers-" + UUID.randomUUID().toString().substring(0, 8);
        client.createCache(cacheId);
        populateEntries(cacheId, ENTRY_COUNT, 600_000L);

        final List<TimerInfo> timers = client.getTimers();
        // getTimers is global (all caches); assert our cache's timers all survived reassembly.
        final Set<String> ours = timers.stream()
                .filter(t -> cacheId.equals(t.getCacheId()))
                .peek(t -> {
                    assertEquals("CACHE", t.getTimerType());
                    assertTrue(t.getDeadline() > 0, "timer deadline should be positive epoch millis");
                })
                .map(TimerInfo::getKey)
                .collect(Collectors.toSet());
        assertEquals(ENTRY_COUNT, ours.size(),
                "every pending timer for " + cacheId + " should be reassembled");
        client.deleteCache(cacheId);
    }

    /** Inserts {@code count} entries (keys {@code k0..k(count-1)}) via chunked bulk ADD_ITEM. */
    private void populateEntries(String cacheId, int count, long ttl) throws Exception {
        for (int start = 0; start < count; start += BULK_CHUNK) {
            final int end = Math.min(start + BULK_CHUNK, count);
            final BulkCacheOpsRequest.Builder request = BulkCacheOpsRequest.builder()
                    .requestId(UUID.randomUUID().toString());
            for (int i = start; i < end; i++) {
                final CacheOperationRequest.Builder op = CacheOperationRequest.builder()
                        .operationType(BulkOperationType.ADD_ITEM)
                        .requestId("add-" + i)
                        .cacheId(cacheId)
                        .key("k" + i)
                        .value(BATCH_VALUE);
                if (ttl > 0) {
                    op.ttl(ttl);
                }
                request.addOperation(op.build());
            }
            client.bulkOps(request.build());
        }
    }

    @Test
    void subscription() throws Exception {
        final String cacheId = "bidi-sub-" + UUID.randomUUID().toString().substring(0, 8);
        client.createCache(cacheId);

        final List<CacheUpdateEvent> events = new CopyOnWriteArrayList<>();
        try (BidiSubscription ignored = client.subscribe(cacheId, ev -> {
            if ("ADD_ITEM".equals(ev.getEventType()) && "sk".equals(ev.getItemKey())) {
                events.add(ev);
            }
        })) {
            Thread.sleep(500);
            client.putItem(cacheId, "sk", "sv");

            final long deadline = System.currentTimeMillis() + 5_000;
            while (events.isEmpty() && System.currentTimeMillis() < deadline) {
                Thread.sleep(50);
            }
            assertTrue(!events.isEmpty(), "expected an ADD_ITEM stream update for key sk");
            assertEquals("sv", events.get(0).getItemValue());
        }
        client.deleteCache(cacheId);
    }

    /** Keyed subscription: filtered to a single key; updates for other keys must not be delivered. */
    @Test
    void keyedSubscription() throws Exception {
        final String cacheId = "bidi-sub-keys-" + UUID.randomUUID().toString().substring(0, 8);
        client.createCache(cacheId);

        final List<String> received = new CopyOnWriteArrayList<>();
        try (BidiSubscription ignored = client.subscribe(cacheId, false, "key1", null, ev -> {
            if ("ADD_ITEM".equals(ev.getEventType()) && ev.getItemKey() != null) {
                received.add(ev.getItemKey());
            }
        })) {
            // Let the server register the subscription routing before publishing.
            Thread.sleep(500);
            client.putItem(cacheId, "key1", "v1");
            client.putItem(cacheId, "key2", "v2");

            // Wait for the (only expected) key1 event, then a grace period to catch any stray key2 event.
            final long deadline = System.currentTimeMillis() + 10_000;
            while (!received.contains("key1") && System.currentTimeMillis() < deadline) {
                Thread.sleep(50);
            }
            Thread.sleep(1000);

            assertTrue(received.contains("key1"), "expected ADD_ITEM for key1, got " + received);
            assertFalse(received.contains("key2"), "key2 should be filtered out, got " + received);
        }
        client.deleteCache(cacheId);
    }

    /** Patch-mode subscription: a {@code patchItem} should surface a {@code PATCH_ITEM} stream update. */
    @Test
    void patchModeSubscription() throws Exception {
        final String cacheId = "bidi-sub-patch-" + UUID.randomUUID().toString().substring(0, 8);
        client.createCache(cacheId);
        client.putItem(cacheId, "doc", "{\"a\":1}");

        final List<CacheUpdateEvent> events = new CopyOnWriteArrayList<>();
        try (BidiSubscription ignored = client.subscribe(cacheId, false, null, "patch", ev -> {
            if ("PATCH_ITEM".equals(ev.getEventType()) && "doc".equals(ev.getItemKey())) {
                events.add(ev);
            }
        })) {
            // Let the server register the subscription routing before publishing.
            Thread.sleep(500);
            client.patchItem(cacheId, "doc", "{\"b\":2}");

            final long deadline = System.currentTimeMillis() + 10_000;
            while (events.isEmpty() && System.currentTimeMillis() < deadline) {
                Thread.sleep(50);
            }
            assertTrue(!events.isEmpty(), "expected a PATCH_ITEM stream update for key doc");
        }
        client.deleteCache(cacheId);
    }

    /** Counter subscription: a counter put should surface a stream update carrying the new value. */
    @Test
    void counterSubscription() throws Exception {
        final String cacheId = "bidi-sub-counter-" + UUID.randomUUID().toString().substring(0, 8);
        client.createCounterCache(cacheId);

        final List<CounterUpdateEvent> events = new CopyOnWriteArrayList<>();
        try (BidiSubscription ignored = client.subscribeCounter(cacheId, ev -> {
            if ("ws-counter".equals(ev.getItemKey())) {
                events.add(ev);
            }
        })) {
            // Let the server register the subscription routing before publishing.
            Thread.sleep(500);
            client.putCounter(cacheId, "ws-counter", 7);

            final long deadline = System.currentTimeMillis() + 10_000;
            while (events.isEmpty() && System.currentTimeMillis() < deadline) {
                Thread.sleep(50);
            }
            assertTrue(!events.isEmpty(), "expected a counter stream update for key ws-counter");
            assertEquals(Long.valueOf(7L), events.get(0).getItemValue());
        }
        client.clearCounterCache(cacheId);
        client.deleteCounterCache(cacheId);
    }
}
