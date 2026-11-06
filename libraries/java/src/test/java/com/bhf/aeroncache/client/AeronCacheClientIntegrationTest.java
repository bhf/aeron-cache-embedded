package com.bhf.aeroncache.client;

import com.bhf.aeroncache.models.*;
import com.fasterxml.jackson.databind.node.ObjectNode;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Assumptions;

import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

import static org.junit.jupiter.api.Assertions.*;

public class AeronCacheClientIntegrationTest {
    private static AeronCacheClient client;

    @BeforeAll
    public static void setup() {
        String baseUrl = System.getenv("AERON_CACHE_BASE_URL");
        String wsUrl = System.getenv("AERON_CACHE_WS_URL");

        Assumptions.assumeTrue(baseUrl != null && !baseUrl.isEmpty(), "Integration tests disabled: AERON_CACHE_BASE_URL not set");
        
        if (wsUrl == null || wsUrl.isEmpty()) {
            wsUrl = baseUrl.replace("http://", "ws://").replace("https://", "wss://");
        }

        client = new AeronCacheClient(baseUrl, wsUrl);
    }

    @Test
    public void testCacheOperations() throws Exception {
        String cacheId = "it-cache-" + UUID.randomUUID().toString();

        CreateResponse createResp = client.createCache(cacheId);
        assertNotNull(createResp);
        assertEquals(cacheId, createResp.getCacheId());

        EmbeddedAeronCache embedded = client.getCache(cacheId);

        // Put an item
        PutItemResponse putResp = embedded.put("key1", "val1");
        assertNotNull(putResp);
        assertEquals("key1", putResp.getKey());

        // Get the item
        GetItemResponse getResp = embedded.get("key1");
        assertNotNull(getResp);
        assertEquals("val1", getResp.getValue());

        // Remove the item
        DeleteItemResponse delResp = embedded.remove("key1");
        assertNotNull(delResp);

        // Get the item again, should handle 404 cleanly based on our previous logic
        GetItemResponse getResp2 = embedded.get("key1");
        assertNotNull(getResp2);
        assertTrue("UNKNOWN_KEY".equals(getResp2.getOperationStatus()) || getResp2.getValue() == null);
    }

    @Test
    public void testWebsocketSubscription() throws Exception {
        String cacheId = "it-ws-" + UUID.randomUUID().toString();
        client.createCache(cacheId);
        EmbeddedAeronCache embedded = client.getCache(cacheId);

        CountDownLatch latch = new CountDownLatch(1);
        CountDownLatch openLatch = new CountDownLatch(1);
        
        AeronCacheSubscriber subscriber = new AeronCacheSubscriber() {
            @Override
            public void onOpen(java.net.http.WebSocket webSocket) {
                super.onOpen(webSocket);
                openLatch.countDown();
            }

            @Override
            public void onAfterUpdate(CacheUpdateEvent event) {
                System.out.println("Received WS event: " + event.getEventType() + " for key: " + event.getItemKey());
                if ("ADD_ITEM".equals(event.getEventType()) && "ws-key".equals(event.getItemKey())) {
                    latch.countDown();
                }
            }

            @Override
            public void onError(java.net.http.WebSocket webSocket, Throwable error) {
                System.err.println("Websocket error:");
                error.printStackTrace();
            }
        };

        ReconnectingWebSocket ws = embedded.subscribe(subscriber);
        
        // Wait for websocket to connect fully
        boolean opened = openLatch.await(5, TimeUnit.SECONDS);
        assertTrue(opened, "Websocket failed to connect within timeout");

        // The client onOpen fires before the server has necessarily registered the subscription in its
        // update routing, so an immediate put can race ahead of it. Give the subscription a moment to
        // settle before publishing the update we expect to observe.
        Thread.sleep(500);

        embedded.put("ws-key", "ws-val");

        boolean received = latch.await(10, TimeUnit.SECONDS);
        assertTrue(received, "Websocket event not received within timeout");

        // Additionally, local cache should be automatically updated
        assertEquals("ws-val", embedded.getLocal("ws-key"));

        embedded.clear();
        ws.close();
    }

    @Test
    public void testWebsocketHydration() throws Exception {
        String cacheId = "it-hydrate-" + UUID.randomUUID().toString();
        client.createCache(cacheId);
        EmbeddedAeronCache preFill = client.getCache(cacheId);
        preFill.put("hydrate-key", "hydrate-val");

        CountDownLatch openLatch = new CountDownLatch(1);
        
        AeronCacheSubscriber subscriber = new AeronCacheSubscriber() {
            @Override
            public void onOpen(java.net.http.WebSocket webSocket) {
                super.onOpen(webSocket);
                openLatch.countDown();
            }
        };

        EmbeddedAeronCache embedded = client.getCache(cacheId);
        ReconnectingWebSocket ws = embedded.subscribe(subscriber, true);
        
        boolean opened = openLatch.await(5, TimeUnit.SECONDS);
        assertTrue(opened, "Websocket failed to connect within timeout");

        // Give it a moment to process hydration events
        Thread.sleep(2000);

        assertEquals("hydrate-val", embedded.getLocal("hydrate-key"), "Local cache should be hydrated with existing data");

        embedded.clear();
        ws.close();
    }

    @Test
    public void testGetAndClearCache() throws Exception {
        
        String cacheId = "it-cache2-" + System.currentTimeMillis();

        client.createCache(cacheId);
        client.putItem(cacheId, "key1", "val1");
        client.putItem(cacheId, "key2", "val2");

        GetCacheResponse getResp = client.getCacheItems(cacheId);
        assertNotNull(getResp);
        assertEquals(2, getResp.getItems().size());

        ClearCacheResponse clearResp = client.clearCache(cacheId);
        assertNotNull(clearResp);
        assertEquals("SUCCESS", clearResp.getOperationStatus());

        GetCacheResponse getResp2 = client.getCacheItems(cacheId);
        assertEquals(0, getResp2.getItems().size());
    }

    @Test
    public void testBulkOperations() throws Exception {
        String cacheId = "it-bulk-" + System.currentTimeMillis();
        String requestId = "req-" + System.currentTimeMillis();

        BulkCacheOpsRequest request = new BulkCacheOpsRequest.Builder()
                .requestId(requestId)
                .addOperation(new CacheOperationRequest.Builder()
                        .operationType(BulkOperationType.CREATE_CACHE)
                        .requestId("op-1")
                        .cacheId(cacheId)
                        .build())
                .addOperation(new CacheOperationRequest.Builder()
                        .operationType(BulkOperationType.ADD_ITEM)
                        .requestId("op-2")
                        .cacheId(cacheId)
                        .key("bulk-key")
                        .value("bulk-val")
                        .build())
                .addOperation(new CacheOperationRequest.Builder()
                        .operationType(BulkOperationType.GET_ITEM)
                        .requestId("op-3")
                        .cacheId(cacheId)
                        .key("bulk-key")
                        .build())
                .build();

        BulkCacheOpsResponse response = client.bulkOps(request);
        assertNotNull(response);
        assertEquals(requestId, response.getRequestId());
        assertEquals(3, response.getOperationResponses().size());

        assertEquals("op-3", response.getOperationResponses().get(2).getRequestId());
        assertEquals("bulk-val", response.getOperationResponses().get(2).getValue());
    }

    @Test
    public void testCounterOperations() throws Exception {
        String cacheId = "it-counter-" + UUID.randomUUID().toString();

        CreateResponse createResp = client.createCounterCache(cacheId);
        assertNotNull(createResp);
        assertEquals(cacheId, createResp.getCacheId());

        EmbeddedCounterCache counters = client.getCounterCache(cacheId);

        PutItemResponse putResp = counters.put("hits", 10);
        assertNotNull(putResp);
        assertEquals("hits", putResp.getKey());

        assertEquals(15L, counters.increment("hits", 5).getValue());
        assertEquals(12L, counters.decrement("hits", 3).getValue());
        assertEquals(100L, counters.set("hits", 100).getValue());
        assertEquals(100L, counters.get("hits").getValue());

        counters.remove("hits");
        counters.clear();
    }

    @Test
    public void testCounterWebsocketSubscription() throws Exception {
        String cacheId = "it-counter-ws-" + UUID.randomUUID().toString();
        client.createCounterCache(cacheId);
        EmbeddedCounterCache counters = client.getCounterCache(cacheId);

        CountDownLatch latch = new CountDownLatch(1);
        CountDownLatch openLatch = new CountDownLatch(1);

        CounterCacheSubscriber subscriber = new CounterCacheSubscriber() {
            @Override
            public void onOpen(java.net.http.WebSocket webSocket) {
                super.onOpen(webSocket);
                openLatch.countDown();
            }

            @Override
            public void onAfterUpdate(CounterUpdateEvent event) {
                if ("ADD_ITEM".equals(event.getEventType()) && "ws-counter".equals(event.getItemKey())) {
                    latch.countDown();
                }
            }
        };

        ReconnectingWebSocket ws = counters.subscribe(subscriber);
        assertTrue(openLatch.await(5, TimeUnit.SECONDS), "Websocket failed to connect within timeout");

        counters.put("ws-counter", 7);

        assertTrue(latch.await(5, TimeUnit.SECONDS), "Websocket event not received within timeout");
        assertEquals(Long.valueOf(7L), counters.getLocal("ws-counter"));

        counters.clear();
        ws.close();
    }

    @Test
    public void testPutTimedCounter() throws Exception {
        String cacheId = "it-counter-timed-" + UUID.randomUUID().toString();
        client.createCounterCache(cacheId);
        EmbeddedCounterCache counters = client.getCounterCache(cacheId);

        PutItemResponse putResp = counters.putTimed("timed-counter", 5, 2000);
        assertNotNull(putResp);
        assertEquals("timed-counter", putResp.getKey());

        assertEquals(5L, counters.get("timed-counter").getValue());

        // Wait for TTL to expire
        Thread.sleep(3000);

        CounterResponse getResp2 = counters.get("timed-counter");
        assertTrue("UNKNOWN_KEY".equals(getResp2.getOperationStatus()) || getResp2.getValue() == 0L);
    }

    @Test
    public void testPatchItem() throws Exception {
        String cacheId = "it-patch-" + UUID.randomUUID().toString();
        client.createCache(cacheId);

        client.putItem(cacheId, "doc", "{\"a\":1}");
        PatchItemResponse patchResp = client.patchItem(cacheId, "doc", "{\"b\":2}");
        assertNotNull(patchResp);
        assertEquals("doc", patchResp.getKey());

        // Deep-merge: both fields should be present after the patch. Reads are eventually
        // consistent on the cluster, so poll until the merged field is visible.
        GetItemResponse getResp = null;
        String value = "";
        for (int i = 0; i < 25; i++) {
            getResp = client.getItem(cacheId, "doc");
            value = getResp.getValue() == null ? "" : getResp.getValue();
            if (value.contains("\"b\":2")) {
                break;
            }
            Thread.sleep(200);
        }
        assertTrue(value.contains("\"a\":1"), "Patched doc should retain original field: " + value);
        assertTrue(value.contains("\"b\":2"), "Patched doc should contain merged field: " + value);
    }

    @Test
    public void testCancelItemRemoval() throws Exception {
        String cacheId = "it-cancel-" + UUID.randomUUID().toString();
        client.createCache(cacheId);
        EmbeddedAeronCache embedded = client.getCache(cacheId);

        // Put a timed item, then cancel its scheduled removal so it survives past the TTL.
        embedded.putTimed("keep-me", "val", 2000);
        CancelItemRemovalResponse cancelResp = client.cancelItemRemoval(cacheId, "keep-me");
        assertNotNull(cancelResp);
        assertEquals("keep-me", cancelResp.getKey());

        Thread.sleep(3000);

        GetItemResponse getResp = client.getItem(cacheId, "keep-me");
        assertEquals("val", getResp.getValue(), "Item should still be present after cancelling its removal");
    }

    @Test
    public void testGetCachesAndStats() throws Exception {
        String cacheId = "it-list-" + UUID.randomUUID().toString();
        client.createCache(cacheId);
        client.putItem(cacheId, "k1", "v1");
        client.putItem(cacheId, "k2", "v2");

        // The newly created cache should be listed. (getCaches() itemCount is a derived,
        // eventually-consistent summary on the backend, so we don't assert on its exact value.)
        java.util.List<CacheDetails> caches = client.getCaches();
        assertNotNull(caches);
        assertTrue(caches.stream().anyMatch(c -> cacheId.equals(c.getCacheId())),
                "Newly created cache should appear in getCaches()");

        CacheStatsResponse stats = client.getStats();
        assertNotNull(stats);
        assertTrue(stats.getTotalCachesCount() >= 1);
    }

    @Test
    public void testGetTimers() throws Exception {
        String cacheId = "it-timers-" + UUID.randomUUID().toString();
        client.createCache(cacheId);
        // A timed entry schedules a pending TTL removal timer.
        client.putTimedItem(cacheId, "ttl-key", "v", 600_000L);

        GetTimersResponse resp = client.getTimers();
        assertNotNull(resp);
        assertNotNull(resp.getTimers());
        TimerInfo timer = resp.getTimers().stream()
                .filter(t -> cacheId.equals(t.getCacheId()) && "ttl-key".equals(t.getKey()))
                .findFirst().orElse(null);
        assertNotNull(timer, "expected a pending timer for " + cacheId + "/ttl-key");
        assertEquals("CACHE", timer.getTimerType());
        assertTrue(timer.getDeadline() > 0, "timer deadline should be a positive epoch millis");
    }

    @Test
    public void testGetCounterItemsAndClear() throws Exception {
        String cacheId = "it-counter-list-" + UUID.randomUUID().toString();
        client.createCounterCache(cacheId);
        EmbeddedCounterCache counters = client.getCounterCache(cacheId);
        counters.put("hits", 10);
        counters.put("misses", 3);

        GetCountersResponse getResp = client.getCounterItems(cacheId);
        assertNotNull(getResp);
        assertEquals(2, getResp.getItems().size());

        ClearCacheResponse clearResp = client.clearCounterCache(cacheId);
        assertNotNull(clearResp);
        assertEquals("SUCCESS", clearResp.getOperationStatus());

        GetCountersResponse getResp2 = client.getCounterItems(cacheId);
        assertEquals(0, getResp2.getItems().size());
    }

    @Test
    public void testCancelCounterItemRemoval() throws Exception {
        String cacheId = "it-counter-cancel-" + UUID.randomUUID().toString();
        client.createCounterCache(cacheId);
        EmbeddedCounterCache counters = client.getCounterCache(cacheId);

        counters.putTimed("keep-me", 5, 2000);
        CancelItemRemovalResponse cancelResp = client.cancelCounterItemRemoval(cacheId, "keep-me");
        assertNotNull(cancelResp);
        assertEquals("keep-me", cancelResp.getKey());

        Thread.sleep(3000);

        assertEquals(5L, counters.get("keep-me").getValue(), "Counter should survive past TTL after cancelling removal");
    }

    @Test
    public void testGetCounterCachesAndStats() throws Exception {
        String cacheId = "it-counter-caches-" + UUID.randomUUID().toString();
        client.createCounterCache(cacheId);
        EmbeddedCounterCache counters = client.getCounterCache(cacheId);
        counters.put("hits", 1);

        java.util.List<CacheDetails> caches = client.getCounterCaches();
        assertNotNull(caches);
        assertTrue(caches.stream().anyMatch(c -> cacheId.equals(c.getCacheId())),
                "Newly created counter cache should appear in getCounterCaches()");

        CacheStatsResponse stats = client.getCounterStats();
        assertNotNull(stats);
        assertTrue(stats.getTotalCachesCount() >= 1);
    }

    @Test
    public void testWebsocketKeyFilter() throws Exception {
        String cacheId = "it-ws-keys-" + UUID.randomUUID().toString();
        client.createCache(cacheId);

        java.util.List<String> received = new java.util.concurrent.CopyOnWriteArrayList<>();
        CountDownLatch openLatch = new CountDownLatch(1);
        CountDownLatch key1Latch = new CountDownLatch(1);

        AeronCacheSubscriber subscriber = new AeronCacheSubscriber() {
            @Override
            public void onOpen(java.net.http.WebSocket webSocket) {
                super.onOpen(webSocket);
                openLatch.countDown();
            }

            @Override
            public void onAfterUpdate(CacheUpdateEvent event) {
                if ("ADD_ITEM".equals(event.getEventType()) && event.getItemKey() != null) {
                    received.add(event.getItemKey());
                    if ("key1".equals(event.getItemKey())) {
                        key1Latch.countDown();
                    }
                }
            }
        };

        // Subscribe filtered to only "key1"
        ReconnectingWebSocket ws = client.subscribe(cacheId, false, "key1", null, subscriber);
        assertTrue(openLatch.await(5, TimeUnit.SECONDS), "Websocket failed to connect within timeout");

        // Let the server register the subscription routing before publishing.
        Thread.sleep(500);

        client.putItem(cacheId, "key1", "v1");
        client.putItem(cacheId, "key2", "v2");

        // Wait for the (only expected) key1 event, then a grace period to catch any stray key2 event.
        assertTrue(key1Latch.await(10, TimeUnit.SECONDS), "Expected ADD_ITEM for key1 within timeout");
        Thread.sleep(1000);

        assertTrue(received.contains("key1"), "Expected key1 event, got " + received);
        assertFalse(received.contains("key2"), "key2 should be filtered out, got " + received);

        client.deleteCache(cacheId);
        ws.close();
    }

    @Test
    public void testWebsocketPatchMode() throws Exception {
        String cacheId = "it-ws-patch-" + UUID.randomUUID().toString();
        client.createCache(cacheId);
        client.putItem(cacheId, "doc", "{\"a\":1}");

        CountDownLatch openLatch = new CountDownLatch(1);
        CountDownLatch patchLatch = new CountDownLatch(1);

        AeronCacheSubscriber subscriber = new AeronCacheSubscriber() {
            @Override
            public void onOpen(java.net.http.WebSocket webSocket) {
                super.onOpen(webSocket);
                openLatch.countDown();
            }

            @Override
            public void onAfterUpdate(CacheUpdateEvent event) {
                if ("PATCH_ITEM".equals(event.getEventType()) && "doc".equals(event.getItemKey())) {
                    patchLatch.countDown();
                }
            }
        };

        ReconnectingWebSocket ws = client.subscribe(cacheId, false, null, "patch", subscriber);
        assertTrue(openLatch.await(5, TimeUnit.SECONDS), "Websocket failed to connect within timeout");

        // Let the server register the subscription routing before publishing.
        Thread.sleep(500);

        client.patchItem(cacheId, "doc", "{\"b\":2}");

        assertTrue(patchLatch.await(10, TimeUnit.SECONDS), "Expected PATCH_ITEM event for doc within timeout");

        client.deleteCache(cacheId);
        ws.close();
    }

    /**
     * Synthetic patch generation driven by regular {@code putItem} writes (no explicit {@code patchItem}).
     * The cache service, as long as a patch-mode subscriber is present, turns ordinary puts into patch
     * traffic: a first insert arrives as a full {@code ADD_ITEM} baseline (nothing to diff against), an
     * overwrite of an existing JSON value arrives as a {@code PATCH_ITEM} delta carrying only the changed
     * fields, an identical (no-op) put is suppressed, and a non-JSON value falls back to a full
     * {@code ADD_ITEM}. This is distinct from {@link #testWebsocketPatchMode()}, which drives the delta with
     * an explicit {@code patchItem} call.
     */
    @Test
    public void testWebsocketSyntheticPatchFromPut() throws Exception {
        String cacheId = "it-ws-synthpatch-" + UUID.randomUUID();
        client.createCache(cacheId);

        CountDownLatch openLatch = new CountDownLatch(1);
        List<CacheUpdateEvent> events = new CopyOnWriteArrayList<>();
        AeronCacheSubscriber subscriber = new AeronCacheSubscriber() {
            @Override
            public void onOpen(java.net.http.WebSocket webSocket) {
                super.onOpen(webSocket);
                openLatch.countDown();
            }

            @Override
            public void onAfterUpdate(CacheUpdateEvent event) {
                if ("doc".equals(event.getItemKey())) {
                    events.add(event);
                }
            }
        };

        // Subscribe in patch mode BEFORE the key exists, so the first put has no prior value to diff
        // against and the server sends it as a full ADD_ITEM baseline.
        ReconnectingWebSocket ws = client.subscribe(cacheId, false, null, "patch", subscriber);
        assertTrue(openLatch.await(5, TimeUnit.SECONDS), "Websocket failed to connect within timeout");
        // Let the server register the subscription routing before publishing.
        Thread.sleep(500);

        // 1) Brand-new key -> full ADD_ITEM baseline.
        client.putItem(cacheId, "doc", "{\"a\":1}");
        awaitAtLeast(events, 1, 10_000);
        assertEquals("ADD_ITEM", events.get(0).getEventType(),
                "first put to a new key should arrive as a full ADD_ITEM baseline");
        assertTrue(events.get(0).getItemValue().contains("\"a\":1"),
                "baseline should carry the full value: " + events.get(0).getItemValue());

        // 2) Overwrite with an added field -> PATCH_ITEM delta (changed field only).
        client.putItem(cacheId, "doc", "{\"a\":1,\"b\":2}");
        awaitAtLeast(events, 2, 10_000);
        assertEquals("PATCH_ITEM", events.get(1).getEventType(),
                "overwriting an existing value should synthesise a PATCH_ITEM delta");
        assertTrue(events.get(1).getItemValue().contains("\"b\":2"),
                "delta should carry the changed field: " + events.get(1).getItemValue());
        assertFalse(events.get(1).getItemValue().contains("\"a\":1"),
                "delta should not carry unchanged fields: " + events.get(1).getItemValue());

        // 3) Identical value -> suppressed (no event). Allow time to confirm nothing further arrives.
        client.putItem(cacheId, "doc", "{\"a\":1,\"b\":2}");
        Thread.sleep(1500);
        assertEquals(2, events.size(),
                "a no-op put (identical value) should be suppressed, not streamed: " + events);

        // 4) Non-JSON value -> full ADD_ITEM fallback (rather than a broken delta).
        client.putItem(cacheId, "doc", "not-json");
        awaitAtLeast(events, 3, 10_000);
        assertEquals("ADD_ITEM", events.get(2).getEventType(),
                "a non-JSON value should fall back to a full ADD_ITEM");
        assertTrue(events.get(2).getItemValue().contains("not-json"),
                "fallback should carry the full value: " + events.get(2).getItemValue());

        client.deleteCache(cacheId);
        ws.close();
    }

    /**
     * End-to-end reconstruction: an {@link EmbeddedObjectCache} subscribed in patch mode rebuilds the full
     * object locally from the server's synthetic ADD_ITEM baseline plus PATCH_ITEM deltas (deep-merged per
     * RFC 7386), with untouched (including nested) fields preserved. Uses only JSON-object values, since the
     * object cache is JSON-objects-only.
     */
    @Test
    public void testObjectCacheReconstructsFromSyntheticPatch() throws Exception {
        String cacheId = "it-objcache-synthpatch-" + UUID.randomUUID();
        client.createCache(cacheId);
        EmbeddedObjectCache objectCache = client.getObjectCache(cacheId);

        CountDownLatch openLatch = new CountDownLatch(1);
        AeronCacheSubscriber subscriber = new AeronCacheSubscriber() {
            @Override
            public void onOpen(java.net.http.WebSocket webSocket) {
                super.onOpen(webSocket);
                openLatch.countDown();
            }
        };

        // Patch mode over the HTTP WebSocket; the object cache deep-merges deltas into its local map.
        ReconnectingWebSocket ws = objectCache.subscribe(subscriber, false, null, "patch");
        assertTrue(openLatch.await(5, TimeUnit.SECONDS), "Websocket failed to connect within timeout");
        Thread.sleep(500);

        // Baseline (subscribe-before-key): the first put arrives as a full ADD_ITEM the cache stores whole.
        objectCache.put("doc", Map.of("a", 1, "nested", Map.of("x", 1)));
        // Overwrite: the server synthesises a PATCH_ITEM delta, which the object cache deep-merges.
        objectCache.put("doc", Map.of("a", 1, "nested", Map.of("x", 1, "y", 2), "c", 3));

        // Poll the local mirror until the delta has been applied (events are eventually consistent).
        ObjectNode local = null;
        long deadline = System.currentTimeMillis() + 10_000;
        while (System.currentTimeMillis() < deadline) {
            local = objectCache.getLocal("doc");
            if (local != null && local.path("c").asInt(-1) == 3) {
                break;
            }
            Thread.sleep(50);
        }

        assertNotNull(local, "object cache should have mirrored the document");
        assertEquals(1, local.path("a").asInt(), "untouched top-level field should survive the merge");
        assertEquals(3, local.path("c").asInt(), "added top-level field should be applied from the delta");
        assertEquals(1, local.path("nested").path("x").asInt(),
                "untouched nested field should survive the merge");
        assertEquals(2, local.path("nested").path("y").asInt(),
                "added nested field should be applied from the delta");

        client.deleteCache(cacheId);
        ws.close();
    }

    /** Poll until {@code events} holds at least {@code n} elements, or fail after {@code timeoutMs}. */
    private static void awaitAtLeast(List<?> events, int n, long timeoutMs) throws InterruptedException {
        long deadline = System.currentTimeMillis() + timeoutMs;
        while (events.size() < n && System.currentTimeMillis() < deadline) {
            Thread.sleep(50);
        }
        assertTrue(events.size() >= n, "expected at least " + n + " event(s), got " + events.size() + ": " + events);
    }

    @Test
    public void testPutTimedItem() throws Exception {
        String cacheId = "it-timed-" + UUID.randomUUID().toString();
        client.createCache(cacheId);
        EmbeddedAeronCache embedded = client.getCache(cacheId);

        // Put a timed item with 2 second TTL
        PutItemResponse putResp = embedded.putTimed("timed-key", "timed-val", 2000);
        assertNotNull(putResp);
        assertEquals("timed-key", putResp.getKey());

        // Get immediately - should exist
        GetItemResponse getResp = embedded.get("timed-key");
        assertEquals("timed-val", getResp.getValue());

        // Wait for TTL to expire (3 seconds)
        Thread.sleep(3000);

        // Get again - should be gone
        GetItemResponse getResp2 = embedded.get("timed-key");
        assertTrue("UNKNOWN_KEY".equals(getResp2.getOperationStatus()) || getResp2.getValue() == null);
    }
}
