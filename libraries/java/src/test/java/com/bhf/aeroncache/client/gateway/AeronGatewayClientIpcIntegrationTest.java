package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.models.BulkCacheOpsRequest;
import com.bhf.aeroncache.models.BulkCacheOpsResponse;
import com.bhf.aeroncache.models.BulkOperationType;
import com.bhf.aeroncache.models.CacheOperationRequest;
import com.bhf.aeroncache.models.CacheOperationResponse;
import com.bhf.aeroncache.models.CacheUpdateEvent;
import com.bhf.aeroncache.models.TimerInfo;
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
 * End-to-end tests against a real Aeron gateway over the {@link TransportMedia#IPC} transport.
 * <p>
 * Enabled only when {@code -Daeron.gateway.ipc.it=true} is set, so it does not run in the default
 * unit-test build. Unlike {@link AeronGatewayClientIntegrationTest} (UDP, own embedded driver), IPC
 * requires this JVM to share the gateway server's media driver, so the directory must be the <em>same</em>
 * one the server is using — resolved the same way the server itself resolves {@code aeron.dir}: the
 * {@code aeron.dir} system property, else the {@code AERON_DIR} environment variable, else {@code "aeron"}.
 * Enable the gateway on the monolith with {@code -Daeron.gateway.enabled=true} (or
 * {@code AERON_GATEWAY_ENABLED=true}) and {@code GATEWAY_TRANSPORT_MEDIA=ipc}.
 * <p>
 * The wire encoding/decoding is identical to the UDP transport (same SBE frames, same commands) — this
 * class exists to prove the IPC channel wiring itself, so it covers a representative subset of
 * operations (single command/response, batched-response accumulation, streaming, bulk) rather than
 * duplicating every case in {@link AeronGatewayClientIntegrationTest}.
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@EnabledIfSystemProperty(named = "aeron.gateway.ipc.it", matches = "true")
class AeronGatewayClientIpcIntegrationTest {

    private AeronGatewayClient client;

    @BeforeAll
    void setUp() {
        final String aeronDir = System.getProperty("aeron.dir",
                System.getenv().getOrDefault("AERON_DIR", "aeron"));
        client = AeronGatewayClient.connectIpc(aeronDir);
        assertTrue(client.awaitConnected(10, TimeUnit.SECONDS), "gateway did not connect over IPC");
    }

    @AfterAll
    void tearDown() {
        CloseHelper.quietClose(client);
    }

    @Test
    void cacheLifecycle() throws Exception {
        final String cacheId = "it-ipc-cache-" + UUID.randomUUID();

        final var created = client.createCache(cacheId);
        assertEquals(cacheId, created.getCacheId());

        final var put = client.putItem(cacheId, "k1", "v1");
        assertEquals("k1", put.getKey());

        final var got = client.getItem(cacheId, "k1");
        assertEquals("v1", got.getValue());

        final var removed = client.deleteItem(cacheId, "k1");
        assertEquals("k1", removed.getKey());

        final var deleted = client.deleteCache(cacheId);
        assertEquals(cacheId, deleted.getCacheId());
    }

    @Test
    void counterLifecycle() throws Exception {
        final String cacheId = "it-ipc-counter-" + UUID.randomUUID();

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
        final String cacheId = "it-ipc-stream-" + UUID.randomUUID();
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
    void getTimersReturnsPendingTimers() throws Exception {
        final String cacheId = "it-ipc-timers-" + UUID.randomUUID();
        client.createCache(cacheId);
        // A timed entry schedules a pending TTL removal timer; getTimers streams (and here
        // accumulates) one or more batches, exercising batched-response reassembly over IPC.
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
    void bulkOpsOverGateway() throws Exception {
        final String cacheId = "it-ipc-bulk-" + UUID.randomUUID();

        final BulkCacheOpsRequest request = BulkCacheOpsRequest.builder()
                .requestId(UUID.randomUUID().toString())
                .addOperation(CacheOperationRequest.builder()
                        .operationType(BulkOperationType.CREATE_CACHE)
                        .requestId("op-create")
                        .cacheId(cacheId)
                        .build())
                .addOperation(CacheOperationRequest.builder()
                        .operationType(BulkOperationType.ADD_ITEM)
                        .requestId("op-add-1")
                        .cacheId(cacheId)
                        .key("bk1")
                        .value("bv1")
                        .build())
                .addOperation(CacheOperationRequest.builder()
                        .operationType(BulkOperationType.GET_ITEM)
                        .requestId("op-get-1")
                        .cacheId(cacheId)
                        .key("bk1")
                        .build())
                .build();

        final BulkCacheOpsResponse response = client.bulkOps(request);
        assertEquals(request.getRequestId(), response.getRequestId());
        assertNotNull(response.getOperationResponses());
        assertEquals(3, response.getOperationResponses().size());

        final CacheOperationResponse getResult = response.getOperationResponses().stream()
                .filter(r -> "op-get-1".equals(r.getRequestId()))
                .findFirst().orElse(null);
        assertNotNull(getResult, "expected a result for op-get-1");
        assertEquals("bv1", getResult.getValue());

        client.deleteCache(cacheId);
    }
}
