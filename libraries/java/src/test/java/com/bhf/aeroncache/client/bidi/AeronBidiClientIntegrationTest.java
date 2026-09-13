package com.bhf.aeroncache.client.bidi;

import com.bhf.aeroncache.models.CacheItem;
import com.bhf.aeroncache.models.CacheUpdateEvent;
import com.bhf.aeroncache.models.CounterItem;
import com.bhf.aeroncache.models.GetCacheResponse;
import com.bhf.aeroncache.models.GetCountersResponse;
import com.bhf.aeroncache.models.StatEntry;
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
}
