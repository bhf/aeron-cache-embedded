package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.models.CounterResponse;
import com.bhf.aeroncache.models.CreateResponse;
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

import java.util.UUID;
import java.util.concurrent.TimeUnit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * End-to-end tests against a real Aeron gateway.
 * <p>
 * Enabled only when {@code -Daeron.gateway.it=true} is set, so it does not run in the default unit-test
 * build. The gateway must be reachable at {@code aeron.gateway.host} (default {@code 127.0.0.1}) on the
 * default request/response ports. Enable the gateway on the monolith with
 * {@code -Daeron.transport.gateway.enabled=true} (or {@code AERON_TRANSPORT_GATEWAY_ENABLED=true}), and
 * pin its endpoints to loopback with {@code GATEWAY_REQUEST_ENDPOINT=127.0.0.1:7075} and
 * {@code GATEWAY_RESPONSE_CONTROL_ENDPOINT=127.0.0.1:7076}.
 * <p>
 * This test launches its own embedded media driver and talks to the gateway over UDP.
 * <p>
 * <b>Scope:</b> these cover the operations the current gateway release acknowledges over Aeron —
 * create / put / get for both caches and counters. The gateway does not yet emit responses for
 * clear, delete/remove, counter increment/decrement/set, or streaming updates; integration coverage
 * for those (and the embedded local-mirror caches, which depend on streaming) will be added once the
 * gateway release acknowledges them. The client already implements all of these operations.
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
    void createPutGet() throws Exception {
        final String cacheId = "it-cache-" + UUID.randomUUID();

        final CreateResponse created = client.createCache(cacheId);
        assertEquals(cacheId, created.getCacheId());

        final PutItemResponse put = client.putItem(cacheId, "k1", "v1");
        assertEquals("k1", put.getKey());

        final GetItemResponse got = client.getItem(cacheId, "k1");
        assertEquals("v1", got.getValue());
    }

    @Test
    void createPutGetCounter() throws Exception {
        final String cacheId = "it-counter-" + UUID.randomUUID();

        final CreateResponse created = client.createCounterCache(cacheId);
        assertEquals(cacheId, created.getCacheId());

        final PutItemResponse put = client.putCounter(cacheId, "hits", 10);
        assertEquals("hits", put.getKey());

        final CounterResponse got = client.getCounter(cacheId, "hits");
        assertEquals(10L, got.getValue());
    }
}
