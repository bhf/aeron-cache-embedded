package com.aeron.cache.client;

import com.aeron.cache.models.*;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.net.http.WebSocket;
import java.util.concurrent.CompletableFuture;

public class AeronCacheClient {
    private final String baseUrl;
    private final String wsUrl;
    private final HttpClient httpClient;
    private final ObjectMapper objectMapper;

    public AeronCacheClient(String baseUrl, String wsUrl) {
        this.baseUrl = baseUrl;
        this.wsUrl = wsUrl;
        this.httpClient = HttpClient.newHttpClient();
        this.objectMapper = new ObjectMapper()
                .configure(com.fasterxml.jackson.databind.DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);
    }

    // --- Sync Operations ---

    
    public GetCacheResponse getCacheItems(String cacheId) throws Exception {
        return getCacheItemsAsync(cacheId).get();
    }

    public ClearCacheResponse clearCache(String cacheId) throws Exception {
        return clearCacheAsync(cacheId).get();
    }

    public CreateResponse createCache(String cacheId) throws Exception {
        String json = "{\"cacheId\":\"" + cacheId + "\"}";
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache"))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), CreateResponse.class);
    }

    public PutItemResponse putItem(String cacheId, String key, String value) throws Exception {
        String json = String.format("{\"key\":\"%s\",\"value\":\"%s\"}", key, value);
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), PutItemResponse.class);
    }

    public GetItemResponse getItem(String cacheId, String key) throws Exception {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId + "/" + key))
                .GET()
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), GetItemResponse.class);
    }

    public DeleteItemResponse deleteItem(String cacheId, String key) throws Exception {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId + "/" + key))
                .DELETE()
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), DeleteItemResponse.class);
    }
    
    public DeleteCacheResponse deleteCache(String cacheId) throws Exception {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId))
                .DELETE()
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), DeleteCacheResponse.class);
    }

    // --- Async Operations ---

    public CompletableFuture<CreateResponse> createCacheAsync(String cacheId) {
        String json = "{\"cacheId\":\"" + cacheId + "\"}";
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache"))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(resp -> {
                    checkStatusAsync(resp);
                    try {
                        return objectMapper.readValue(resp.body(), CreateResponse.class);
                    } catch (Exception e) {
                        throw new RuntimeException(e);
                    }
                });
    }

    public CompletableFuture<PutItemResponse> putItemAsync(String cacheId, String key, String value) {
        String json = String.format("{\"key\":\"%s\",\"value\":\"%s\"}", key, value);
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(resp -> {
                    checkStatusAsync(resp);
                    try {
                        return objectMapper.readValue(resp.body(), PutItemResponse.class);
                    } catch (Exception e) {
                        throw new RuntimeException(e);
                    }
                });
    }

    public CompletableFuture<GetItemResponse> getItemAsync(String cacheId, String key) {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId + "/" + key))
                .GET()
                .build();
        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(resp -> {
                    checkStatusAsync(resp);
                    try {
                        return objectMapper.readValue(resp.body(), GetItemResponse.class);
                    } catch (Exception e) {
                        throw new RuntimeException(e);
                    }
                });
    }

    public CompletableFuture<DeleteItemResponse> deleteItemAsync(String cacheId, String key) {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId + "/" + key))
                .DELETE()
                .build();
        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(resp -> {
                    checkStatusAsync(resp);
                    try {
                        return objectMapper.readValue(resp.body(), DeleteItemResponse.class);
                    } catch (Exception e) {
                        throw new RuntimeException(e);
                    }
                });
    }
    
    
    public CompletableFuture<GetCacheResponse> getCacheItemsAsync(String cacheId) {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId))
                .GET()
                .build();
                
        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(response -> response.body())
                .thenApply(body -> {
                    try {
                        return objectMapper.readValue(body, GetCacheResponse.class);
                    } catch (Exception e) {
                        throw new RuntimeException(e);
                    }
                });
    }

    public CompletableFuture<ClearCacheResponse> clearCacheAsync(String cacheId) {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId))
                .method("PATCH", HttpRequest.BodyPublishers.noBody())
                .build();

        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(response -> response.body())
                .thenApply(body -> {
                    try {
                        return objectMapper.readValue(body, ClearCacheResponse.class);
                    } catch (Exception e) {
                        throw new RuntimeException(e);
                    }
                });
    }

    public CompletableFuture<DeleteCacheResponse> deleteCacheAsync(String cacheId) {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId))
                .DELETE()
                .build();
        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(resp -> {
                    checkStatusAsync(resp);
                    try {
                        return objectMapper.readValue(resp.body(), DeleteCacheResponse.class);
                    } catch (Exception e) {
                        throw new RuntimeException(e);
                    }
                });
    }

    // --- WebSocket ---
    
    public ReconnectingWebSocket subscribe(String cacheId, WebSocket.Listener listener) {
        return new ReconnectingWebSocket(httpClient, URI.create(wsUrl + "/api/ws/v1/cache/" + cacheId), listener);
    }

    public EmbeddedAeronCache getCache(String cacheId) {
        return new EmbeddedAeronCache(this, cacheId);
    }


    private void checkStatus(HttpResponse<String> response) {
        if (response.statusCode() >= 400 && response.statusCode() != 400 && response.statusCode() != 401 && response.statusCode() != 404) {
            throw new RuntimeException("Http Error: " + response.statusCode() + " Body: " + response.body());
        }
    }

    private void checkStatusAsync(HttpResponse<String> response) {
        if (response.statusCode() >= 400 && response.statusCode() != 400 && response.statusCode() != 401 && response.statusCode() != 404) {
            throw new RuntimeException("Http Error: " + response.statusCode() + " Body: " + response.body());
        }
    }
}
