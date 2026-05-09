package com.aeron.cache.client;

import com.aeron.cache.models.*;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Assumptions;

import java.util.UUID;
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
        assertTrue(getResp2.getOperationStatus() == OperationStatus.UNKNOWN_KEY || getResp2.getValue() == null);
    }

    @Test
    public void testWebsocketSubscription() throws Exception {
        String cacheId = "it-ws-" + UUID.randomUUID().toString();
        client.createCache(cacheId);
        EmbeddedAeronCache embedded = client.getCache(cacheId);

        CountDownLatch latch = new CountDownLatch(1);
        AeronCacheSubscriber subscriber = new AeronCacheSubscriber() {
            @Override
            public void onAfterUpdate(CacheUpdateEvent event) {
                if ("ADD_ITEM".equals(event.getEventType()) && "ws-key".equals(event.getItemKey())) {
                    latch.countDown();
                }
            }
        };

        ReconnectingWebSocket ws = embedded.subscribe(subscriber);
        
        // Allow some time for websocket to connect
        Thread.sleep(1000);

        embedded.put("ws-key", "ws-val");

        boolean received = latch.await(5, TimeUnit.SECONDS);
        assertTrue(received, "Websocket event not received within timeout");

        // Additionally, local cache should be automatically updated
        assertEquals("ws-val", embedded.getLocal("ws-key"));

        embedded.clear();
        ws.close();
    }
}
