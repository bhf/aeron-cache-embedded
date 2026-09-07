package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.client.EmbeddedAeronCache;
import com.bhf.aeroncache.client.EmbeddedCounterCache;
import com.bhf.aeroncache.models.CacheUpdateEvent;
import com.bhf.aeroncache.models.CounterResponse;
import com.bhf.aeroncache.models.CounterUpdateEvent;
import com.bhf.aeroncache.models.CreateResponse;
import com.bhf.aeroncache.models.DeleteCacheResponse;
import com.bhf.aeroncache.models.DeleteItemResponse;
import com.bhf.aeroncache.models.GetCacheResponse;
import com.bhf.aeroncache.models.GetItemResponse;
import com.bhf.aeroncache.models.PutItemResponse;
import io.aeron.driver.MediaDriver;
import io.aeron.driver.ThreadingMode;
import org.agrona.CloseHelper;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;

import java.util.List;
import java.util.UUID;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.TimeUnit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * End-to-end tests against a real Aeron gateway, covering the full cache and counter operation set as
 * well as streaming subscriptions and the local-mirroring embedded caches.
 * <p>
 * Enabled only when {@code -Daeron.gateway.it=true} is set, so it does not run in the default unit-test
 * build. The gateway must be reachable at {@code aeron.gateway.host} (default {@code 127.0.0.1}) on the
 * default request/response ports. Enable the gateway on the monolith with
 * {@code -Daeron.transport.gateway.enabled=true} (or {@code AERON_TRANSPORT_GATEWAY_ENABLED=true}), and
 * pin its endpoints to loopback with {@code GATEWAY_REQUEST_ENDPOINT=127.0.0.1:7075} and
 * {@code GATEWAY_RESPONSE_CONTROL_ENDPOINT=127.0.0.1:7076}.
 * <p>
 * This test launches its own embedded media driver and talks to the gateway over UDP.
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@EnabledIfSystemProperty(named = "aeron.gateway.it", matches = "true")
class AeronGatewayClientIntegrationTest {

    private MediaDriver mediaDriver;
    private AeronGatewayClient client;

    @BeforeAll
    void setUp() {
        final String host = System.getProperty("aeron.gateway.host", "127.0.0.1");
        mediaDriver = MediaDriver.launchEmbedded(new MediaDriver.Context()
                .threadingMode(ThreadingMode.SHARED)
                .dirDeleteOnStart(true)
                .dirDeleteOnShutdown(true));
        client = AeronGatewayClient.connect(mediaDriver.aeronDirectoryName(), host);
        assertTrue(client.awaitConnected(10, TimeUnit.SECONDS), "gateway did not connect");
    }

    @AfterAll
    void tearDown() {
        CloseHelper.quietClose(client);
        CloseHelper.quietClose(mediaDriver);
    }

    @Test
    void cacheLifecycle() throws Exception {
        final String cacheId = "it-cache-" + UUID.randomUUID();

        final CreateResponse created = client.createCache(cacheId);
        assertEquals(cacheId, created.getCacheId());

        final PutItemResponse put = client.putItem(cacheId, "k1", "v1");
        assertEquals("k1", put.getKey());

        final GetItemResponse got = client.getItem(cacheId, "k1");
        assertEquals("v1", got.getValue());

        final DeleteItemResponse removed = client.deleteItem(cacheId, "k1");
        assertEquals("k1", removed.getKey());

        final DeleteCacheResponse deleted = client.deleteCache(cacheId);
        assertEquals(cacheId, deleted.getCacheId());
    }

    @Test
    void getCacheItemsAndClear() throws Exception {
        final String cacheId = "it-entries-" + UUID.randomUUID();
        client.createCache(cacheId);
        client.putItem(cacheId, "a", "1");
        client.putItem(cacheId, "b", "2");

        final GetCacheResponse all = client.getCacheItems(cacheId);
        assertEquals(2, all.getItems().size());

        client.clearCache(cacheId);

        final GetCacheResponse afterClear = client.getCacheItems(cacheId);
        assertTrue(afterClear.getItems().isEmpty(), "cache should be empty after clear");

        client.deleteCache(cacheId);
    }

    @Test
    void getStatsReturnsCacheStats() throws Exception {
        final String cacheId = "it-stats-" + UUID.randomUUID();
        client.createCache(cacheId);
        client.putItem(cacheId, "s1", "v1");
        client.putItem(cacheId, "s2", "v2");

        final List<GatewayStat> stats = client.getStats();
        final GatewayStat stat = stats.stream()
                .filter(s -> cacheId.equals(s.cacheId()))
                .findFirst().orElse(null);
        assertNotNull(stat, "expected stats for cache " + cacheId);
        assertEquals(2, stat.size());

        client.deleteCache(cacheId);
    }

    @Test
    void counterLifecycle() throws Exception {
        final String cacheId = "it-counter-" + UUID.randomUUID();

        client.createCounterCache(cacheId);
        client.putCounter(cacheId, "hits", 10);

        assertEquals(15, client.incrementCounter(cacheId, "hits", 5).getValue());
        assertEquals(12, client.decrementCounter(cacheId, "hits", 3).getValue());
        assertEquals(100, client.setCounter(cacheId, "hits", 100).getValue());
        assertEquals(100, client.getCounter(cacheId, "hits").getValue());

        client.deleteCounter(cacheId, "hits");
        client.deleteCounterCache(cacheId);
    }

    @Test
    void streamingUpdates() throws Exception {
        final String cacheId = "it-stream-" + UUID.randomUUID();
        client.createCache(cacheId);

        final List<CacheUpdateEvent> events = new CopyOnWriteArrayList<>();
        try (GatewaySubscription ignored = client.subscribe(cacheId, events::add)) {
            Thread.sleep(500);
            client.putItem(cacheId, "sk", "sv");

            final long deadline = System.currentTimeMillis() + 5_000;
            while (events.isEmpty() && System.currentTimeMillis() < deadline) {
                Thread.sleep(50);
            }
            assertTrue(events.stream().anyMatch(e -> "sk".equals(e.getItemKey())), "expected an update for key sk");
        }
        client.deleteCache(cacheId);
    }

    @Test
    void counterStreamingUpdates() throws Exception {
        final String cacheId = "it-cstream-" + UUID.randomUUID();
        client.createCounterCache(cacheId);

        final List<CounterUpdateEvent> events = new CopyOnWriteArrayList<>();
        try (GatewaySubscription ignored = client.subscribeCounter(cacheId, events::add)) {
            Thread.sleep(500);
            client.putCounter(cacheId, "ck", 7);

            final long deadline = System.currentTimeMillis() + 5_000;
            while (events.isEmpty() && System.currentTimeMillis() < deadline) {
                Thread.sleep(50);
            }
            final CounterUpdateEvent event = events.stream()
                    .filter(e -> "ck".equals(e.getItemKey()))
                    .findFirst().orElse(null);
            assertNotNull(event, "expected a counter update for key ck");
        }
        client.deleteCounterCache(cacheId);
    }

    @Test
    void embeddedCacheMirrorsOverAeron() throws Exception {
        final String cacheId = "it-embedded-" + UUID.randomUUID();
        client.createCache(cacheId);

        final EmbeddedAeronCache embedded = client.getCache(cacheId);
        try (AutoCloseable ignored = embedded.subscribe(e -> { })) {
            Thread.sleep(500);
            embedded.put("ek", "ev");

            final long deadline = System.currentTimeMillis() + 5_000;
            while (embedded.getLocal("ek") == null && System.currentTimeMillis() < deadline) {
                Thread.sleep(50);
            }
            assertEquals("ev", embedded.getLocal("ek"));
        }
        client.deleteCache(cacheId);
    }

    @Test
    void embeddedCounterCacheMirrorsOverAeron() throws Exception {
        final String cacheId = "it-embedded-counter-" + UUID.randomUUID();
        client.createCounterCache(cacheId);

        final EmbeddedCounterCache embedded = client.getCounterCache(cacheId);
        try (AutoCloseable ignored = embedded.subscribe(e -> { })) {
            Thread.sleep(500);
            embedded.put("ec", 42);

            final long deadline = System.currentTimeMillis() + 5_000;
            while (embedded.getLocal("ec") == null && System.currentTimeMillis() < deadline) {
                Thread.sleep(50);
            }
            assertEquals(42L, embedded.getLocal("ec"));
        }
        client.deleteCounterCache(cacheId);
    }
}
