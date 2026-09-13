package com.bhf.aeroncache.client;

import com.bhf.aeroncache.models.CreateResponse;
import com.bhf.aeroncache.models.GetItemResponse;
import com.bhf.aeroncache.models.PutItemResponse;
import com.bhf.aeroncache.models.ClearCacheResponse;
import com.bhf.aeroncache.models.GetCacheResponse;
import com.bhf.aeroncache.models.BulkCacheOpsRequest;
import com.bhf.aeroncache.models.BulkCacheOpsResponse;
import com.bhf.aeroncache.models.PatchItemResponse;
import com.bhf.aeroncache.models.CancelItemRemovalResponse;
import com.bhf.aeroncache.models.CacheDetails;
import com.bhf.aeroncache.models.CacheStatsResponse;
import com.github.tomakehurst.wiremock.WireMockServer;
import com.github.tomakehurst.wiremock.client.WireMock;
import com.github.tomakehurst.wiremock.core.WireMockConfiguration;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import static com.github.tomakehurst.wiremock.client.WireMock.*;
import static org.junit.jupiter.api.Assertions.*;

public class AeronCacheClientTest {

    private static WireMockServer wireMockServer;
    private AeronCacheClient client;

    @BeforeAll
    static void startServer() {
        wireMockServer = new WireMockServer(WireMockConfiguration.wireMockConfig().dynamicPort());
        wireMockServer.start();
        WireMock.configureFor("localhost", wireMockServer.port());
    }

    @AfterAll
    static void stopServer() {
        wireMockServer.stop();
    }

    @BeforeEach
    void setUp() {
        wireMockServer.resetAll();
        client = new AeronCacheClient(wireMockServer.baseUrl(), "ws://localhost:7071");
    }

    @Test
    public void testCreateCache() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/cache"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-cache\",\"status\":\"CREATED\"}")));

        CreateResponse response = client.createCache("test-cache");
        assertNotNull(response);
    }

    @Test
    public void testPutItem() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/cache/test-cache"))
                .withRequestBody(matchingJsonPath("$.key", equalTo("my-key")))
                .withRequestBody(matchingJsonPath("$.value", equalTo("my-value")))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"success\":true}")));

        PutItemResponse response = client.putItem("test-cache", "my-key", "my-value");
        assertNotNull(response);
    }

    @Test
    public void testPutTimedItem() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/cache/timed/test-cache"))
                .withRequestBody(matchingJsonPath("$.key", equalTo("my-key")))
                .withRequestBody(matchingJsonPath("$.value", equalTo("my-value")))
                .withRequestBody(matchingJsonPath("$.ttl", equalTo("1000")))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"success\":true}")));

        PutItemResponse response = client.putTimedItem("test-cache", "my-key", "my-value", 1000L);
        assertNotNull(response);
    }

    @Test
    public void testGetItem() throws Exception {
        stubFor(get(urlEqualTo("/api/v1/cache/test-cache/my-key"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"key\":\"my-key\",\"value\":\"my-value\"}")));

        GetItemResponse response = client.getItem("test-cache", "my-key");
        assertNotNull(response);
    }
    
    @Test
    public void testHttpErrorThrowsException() {
        stubFor(get(urlEqualTo("/api/v1/cache/test-cache/my-key"))
                .willReturn(aResponse().withStatus(500).withBody("Internal Server Error")));

        Exception exception = assertThrows(RuntimeException.class, () -> client.getItem("test-cache", "my-key"));
        assertTrue(exception.getMessage().contains("Http Error: 500"));
    }

    @Test
    public void testBulkOps() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/cache/bulkops"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"requestId\":\"req-1\",\"operationResponses\":[]}")));

        BulkCacheOpsResponse response = client.bulkOps(BulkCacheOpsRequest.builder()
                .requestId("req-1")
                .operations(java.util.Collections.emptyList())
                .build());
        assertNotNull(response);
        assertEquals("req-1", response.getRequestId());
    }

    @Test
    public void testAllowBusinessLogicHttpErrors() throws Exception {
        stubFor(get(urlEqualTo("/api/v1/cache/test-cache/non-existent-key"))
                .willReturn(aResponse()
                        .withStatus(404)
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"operationStatus\":\"UNKNOWN_KEY\"}")));

        GetItemResponse response = client.getItem("test-cache", "non-existent-key");
        assertNotNull(response);
        assertEquals("UNKNOWN_KEY", response.getOperationStatus());
    }

    @Test
    public void testGetCacheItems() throws Exception {
        stubFor(get(urlEqualTo("/api/v1/cache/test-cache-get"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-cache-get\",\"operationStatus\":\"SUCCESS\",\"items\":[{\"key\":\"k1\",\"value\":\"v1\"}]}")));

        GetCacheResponse response = client.getCacheItems("test-cache-get");
        assertNotNull(response);
        assertEquals("test-cache-get", response.getCacheId());
        assertNotNull(response.getItems());
        assertEquals(1, response.getItems().size());
        assertEquals("k1", response.getItems().get(0).getKey());
    }

    @Test
    public void testClearCache() throws Exception {
        stubFor(patch(urlEqualTo("/api/v1/cache/test-cache-clear"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-cache-clear\",\"operationStatus\":\"SUCCESS\"}")));

        ClearCacheResponse response = client.clearCache("test-cache-clear");
        assertNotNull(response);
        assertEquals("SUCCESS", response.getOperationStatus());
    }

    @Test
    public void testPatchItem() throws Exception {
        stubFor(patch(urlEqualTo("/api/v1/cache/test-cache/doc"))
                .withRequestBody(matchingJsonPath("$.value", equalTo("{\"b\":2}")))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-cache\",\"key\":\"doc\",\"operationStatus\":\"SUCCESS\"}")));

        PatchItemResponse response = client.patchItem("test-cache", "doc", "{\"b\":2}");
        assertNotNull(response);
        assertEquals("test-cache", response.getCacheId());
        assertEquals("doc", response.getKey());
        assertEquals("SUCCESS", response.getOperationStatus());
    }

    @Test
    public void testCancelItemRemoval() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/cache/test-cache/my-key/cancel-removal"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-cache\",\"key\":\"my-key\",\"operationStatus\":\"SUCCESS\"}")));

        CancelItemRemovalResponse response = client.cancelItemRemoval("test-cache", "my-key");
        assertNotNull(response);
        assertEquals("my-key", response.getKey());
        assertEquals("SUCCESS", response.getOperationStatus());
    }

    @Test
    public void testGetCaches() throws Exception {
        stubFor(get(urlEqualTo("/api/v1/caches"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("[{\"cacheId\":\"c1\",\"itemCount\":2},{\"cacheId\":\"c2\",\"itemCount\":5}]")));

        java.util.List<CacheDetails> caches = client.getCaches();
        assertNotNull(caches);
        assertEquals(2, caches.size());
        assertEquals("c1", caches.get(0).getCacheId());
        assertEquals(2L, caches.get(0).getItemCount());
        assertEquals(5L, caches.get(1).getItemCount());
    }

    @Test
    public void testGetStats() throws Exception {
        stubFor(get(urlEqualTo("/api/v1/stats"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"totalOpsCount\":10,\"totalCachesCount\":2,\"totalItemsCount\":7,\"errorCount\":1}")));

        CacheStatsResponse stats = client.getStats();
        assertNotNull(stats);
        assertEquals(10, stats.getTotalOpsCount());
        assertEquals(2, stats.getTotalCachesCount());
        assertEquals(7, stats.getTotalItemsCount());
        assertEquals(1, stats.getErrorCount());
    }

    @Test
    public void testSubscribeHydrate() {
        ReconnectingWebSocket ws = client.subscribe("cache1", true, new java.net.http.WebSocket.Listener() {});
        assertNotNull(ws);
        ws.close();
    }

    @Test
    public void testSubscribeMulti() {
        ReconnectingWebSocket ws = client.subscribe("cache1,cache2", false, new java.net.http.WebSocket.Listener() {});
        assertNotNull(ws);
        ws.close();
    }

    @Test
    public void testWsQueryBuilder() {
        // No params -> empty query string
        assertEquals("", AeronCacheClient.wsQuery(null, null));

        // keys only -> comma URL-encoded
        assertEquals("?keys=a%2Cb", AeronCacheClient.wsQuery("a,b", null));

        // keys with cacheId:key tokens -> colon and comma URL-encoded
        assertEquals("?keys=c1%3Ak1%2Ck2", AeronCacheClient.wsQuery("c1:k1,k2", null));

        // mode only
        assertEquals("?mode=patch", AeronCacheClient.wsQuery(null, "patch"));

        // both keys and mode
        String q = AeronCacheClient.wsQuery("k", "full");
        assertTrue(q.startsWith("?"));
        assertTrue(q.contains("keys=k"));
        assertTrue(q.contains("mode=full"));
    }

}