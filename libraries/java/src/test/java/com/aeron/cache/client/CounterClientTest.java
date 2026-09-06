package com.aeron.cache.client;

import com.aeron.cache.models.*;
import com.github.tomakehurst.wiremock.WireMockServer;
import com.github.tomakehurst.wiremock.client.WireMock;
import com.github.tomakehurst.wiremock.core.WireMockConfiguration;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.util.List;

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
    public void testGetCounter() throws Exception {
        stubFor(get(urlEqualTo("/api/v1/counters/test-counter/hits"))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"cacheId\":\"test-counter\",\"key\":\"hits\",\"value\":42,\"operationStatus\":\"SUCCESS\"}")));

        CounterResponse response = client.getCounter("test-counter", "hits");
        assertNotNull(response);
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

        DeleteItemResponse delItem = client.deleteCounter("test-counter", "hits");
        assertEquals("hits", delItem.getKey());

        DeleteCacheResponse delCache = client.deleteCounterCache("test-counter");
        assertEquals("test-counter", delCache.getCacheId());
    }

    @Test
    public void testBulkOps() throws Exception {
        stubFor(post(urlEqualTo("/api/v1/cache/bulkops"))
                .withRequestBody(matchingJsonPath("$.operations[0].operationType", equalTo("CREATE_COUNTER_CACHE")))
                .willReturn(aResponse()
                        .withHeader("Content-Type", "application/json")
                        .withBody("{\"requestId\":\"req-1\",\"operationResponses\":["
                                + "{\"requestId\":\"op-1\",\"status\":\"SUCCESS\",\"cacheId\":\"test-counter\"},"
                                + "{\"requestId\":\"op-2\",\"status\":\"SUCCESS\",\"cacheId\":\"test-counter\",\"key\":\"hits\",\"value\":\"5\"}"
                                + "]}")));

        CacheOperationRequest op1 = new CacheOperationRequest();
        op1.setOperationType(BulkOperationType.CREATE_COUNTER_CACHE);
        op1.setRequestId("op-1");
        op1.setCacheId("test-counter");

        CacheOperationRequest op2 = new CacheOperationRequest();
        op2.setOperationType(BulkOperationType.INCREMENT_COUNTER);
        op2.setRequestId("op-2");
        op2.setCacheId("test-counter");
        op2.setKey("hits");
        op2.setCounterValue(5L);

        BulkCacheOpsRequest request = new BulkCacheOpsRequest("req-1", List.of(op1, op2));
        BulkCacheOpsResponse response = client.bulkOps(request);

        assertEquals("req-1", response.getRequestId());
        assertEquals(2, response.getOperationResponses().size());
        assertEquals(OperationStatus.SUCCESS, response.getOperationResponses().get(0).getStatus());
        assertEquals("5", response.getOperationResponses().get(1).getValue());
    }

    @Test
    public void testCounterHttpErrorThrows() {
        stubFor(get(urlEqualTo("/api/v1/counters/test-counter/hits"))
                .willReturn(aResponse().withStatus(500).withBody("Internal Server Error")));

        Exception exception = assertThrows(RuntimeException.class, () -> client.getCounter("test-counter", "hits"));
        assertTrue(exception.getMessage().contains("Http Error: 500"));
    }
}
