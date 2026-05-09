package com.aeron.cache.client;

import com.aeron.cache.models.CreateResponse;
import com.aeron.cache.models.GetItemResponse;
import com.aeron.cache.models.PutItemResponse;
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
}