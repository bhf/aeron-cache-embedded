package com.bhf.aeroncache.client;

import com.bhf.aeroncache.models.*;
import com.github.tomakehurst.wiremock.WireMockServer;
import com.github.tomakehurst.wiremock.client.WireMock;
import com.github.tomakehurst.wiremock.core.WireMockConfiguration;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import static com.github.tomakehurst.wiremock.client.WireMock.*;
import static org.junit.jupiter.api.Assertions.*;

public class CounterClientTest {

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
    public void testCreateCounterCache() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/counters/"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-counter\",\"operationStatus\":\"SUCCESS\"}")));

        CreateResponse response = client.createCounterCache("test-counter");
        assertNotNull(response);
        assertEquals("test-counter", response.getCacheId());
    }

    @Test
    public void testPutCounter() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/counters/test-counter"))
                .withRequestBody(matchingJsonPath("$.key", equalTo("hits")))
                .withRequestBody(matchingJsonPath("$.value", equalTo("42")))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-counter\",\"key\":\"hits\",\"operationStatus\":\"SUCCESS\"}")));

        PutItemResponse response = client.putCounter("test-counter", "hits", 42);
        assertNotNull(response);
        assertEquals("hits", response.getKey());
    }

    @Test
    public void testPutTimedCounter() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/counters/timed/test-counter"))
                .withRequestBody(matchingJsonPath("$.key", equalTo("hits")))
                .withRequestBody(matchingJsonPath("$.value", equalTo("42")))
                .withRequestBody(matchingJsonPath("$.ttl", equalTo("1000")))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-counter\",\"key\":\"hits\",\"operationStatus\":\"SUCCESS\"}")));

        PutItemResponse response = client.putTimedCounter("test-counter", "hits", 42, 1000L);
        assertNotNull(response);
        assertEquals("hits", response.getKey());
    }

    @Test
    public void testGetCounter() throws Exception {
        stubFor(get(urlEqualTo("/api/v1/counters/test-counter/hits"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-counter\",\"key\":\"hits\",\"value\":42,\"operationStatus\":\"SUCCESS\"}")));

        CounterResponse response = client.getCounter("test-counter", "hits");
        assertEquals(42L, response.getValue());
    }

    @Test
    public void testIncrementCounter() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/counters/increment/test-counter"))
                .withRequestBody(matchingJsonPath("$.amount", equalTo("1")))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-counter\",\"key\":\"hits\",\"value\":43,\"operationStatus\":\"SUCCESS\"}")));

        CounterResponse response = client.incrementCounter("test-counter", "hits", 1);
        assertEquals(43L, response.getValue());
    }

    @Test
    public void testDecrementCounter() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/counters/decrement/test-counter"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-counter\",\"key\":\"hits\",\"value\":41,\"operationStatus\":\"SUCCESS\"}")));

        CounterResponse response = client.decrementCounter("test-counter", "hits", 1);
        assertEquals(41L, response.getValue());
    }

    @Test
    public void testSetCounter() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/counters/set/test-counter"))
                .withRequestBody(matchingJsonPath("$.value", equalTo("100")))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-counter\",\"key\":\"hits\",\"value\":100,\"operationStatus\":\"SUCCESS\"}")));

        CounterResponse response = client.setCounter("test-counter", "hits", 100);
        assertEquals(100L, response.getValue());
    }

    @Test
    public void testDeleteCounterAndCache() throws Exception {
        stubFor(delete(urlEqualTo("/api/v1/counters/test-counter/hits"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-counter\",\"key\":\"hits\",\"operationStatus\":\"SUCCESS\"}")));
        stubFor(delete(urlEqualTo("/api/v1/counters/test-counter"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-counter\",\"operationStatus\":\"SUCCESS\"}")));

        assertEquals("hits", client.deleteCounter("test-counter", "hits").getKey());
        assertEquals("test-counter", client.deleteCounterCache("test-counter").getCacheId());
    }

    @Test
    public void testBulkOpsWithCounter() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/cache/bulkops"))
                .withRequestBody(matchingJsonPath("$.operations[1].operationType", equalTo("INCREMENT_COUNTER")))
                .withRequestBody(matchingJsonPath("$.operations[1].counterValue", equalTo("5")))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"requestId\":\"req-1\",\"operationResponses\":["
                                + "{\"requestId\":\"op-1\",\"status\":\"SUCCESS\",\"cacheId\":\"test-counter\"},"
                                + "{\"requestId\":\"op-2\",\"status\":\"SUCCESS\",\"cacheId\":\"test-counter\",\"key\":\"hits\",\"value\":\"5\"}"
                                + "]}")));

        BulkCacheOpsRequest request = BulkCacheOpsRequest.builder()
                .requestId("req-1")
                .addOperation(CacheOperationRequest.builder()
                        .operationType(BulkOperationType.CREATE_COUNTER_CACHE)
                        .requestId("op-1")
                        .cacheId("test-counter")
                        .build())
                .addOperation(CacheOperationRequest.builder()
                        .operationType(BulkOperationType.INCREMENT_COUNTER)
                        .requestId("op-2")
                        .cacheId("test-counter")
                        .key("hits")
                        .counterValue(5L)
                        .build())
                .build();

        BulkCacheOpsResponse response = client.bulkOps(request);
        assertEquals("req-1", response.getRequestId());
        assertEquals(2, response.getOperationResponses().size());
        assertEquals("5", response.getOperationResponses().get(1).getValue());
    }

    @Test
    public void testCounterHttpErrorThrows() {
        stubFor(get(urlEqualTo("/api/v1/counters/test-counter/hits"))
                .willReturn(aResponse().withStatus(500).withBody("Internal Server Error")));

        Exception exception = assertThrows(RuntimeException.class, () -> client.getCounter("test-counter", "hits"));
        assertTrue(exception.getMessage().contains("Http Error: 500"));
    }

    @Test
    public void testSubscribeCounterHydrateAndMulti() {
        ReconnectingWebSocket ws1 = client.subscribeCounter("counter1", true, new java.net.http.WebSocket.Listener() {});
        assertNotNull(ws1);
        ws1.close();

        ReconnectingWebSocket ws2 = client.subscribeCounter("counter1,counter2", false, new java.net.http.WebSocket.Listener() {});
        assertNotNull(ws2);
        ws2.close();
    }
}
