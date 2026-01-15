package com.aeron.cache.client;

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

    public AeronCacheClient(String baseUrl, String wsUrl) {
        this.baseUrl = baseUrl;
        this.wsUrl = wsUrl;
        this.httpClient = HttpClient.newHttpClient();
    }

    // --- Sync Operations ---

    public String createCache(String cacheId) throws Exception {
        String json = "{\"cacheId\":\"" + cacheId + "\"}";
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache"))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return response.body();
    }

    public String putItem(String cacheId, String key, String value) throws Exception {
        String json = String.format("{\"cacheId\":\"%s\",\"key\":\"%s\",\"value\":\"%s\"}", cacheId, key, value);
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return response.body();
    }

    public String getItem(String cacheId, String key) throws Exception {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId + "/" + key))
                .GET()
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return response.body();
    }

    public String deleteItem(String cacheId, String key) throws Exception {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId + "/" + key))
                .DELETE()
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return response.body();
    }
    
    public String deleteCache(String cacheId) throws Exception {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId))
                .DELETE()
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return response.body();
    }

    // --- Async Operations ---

    public CompletableFuture<String> createCacheAsync(String cacheId) {
        String json = "{\"cacheId\":\"" + cacheId + "\"}";
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache"))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(this::checkStatusAsync);
    }

    public CompletableFuture<String> putItemAsync(String cacheId, String key, String value) {
        String json = String.format("{\"cacheId\":\"%s\",\"key\":\"%s\",\"value\":\"%s\"}", cacheId, key, value);
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(this::checkStatusAsync);
    }

    public CompletableFuture<String> getItemAsync(String cacheId, String key) {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId + "/" + key))
                .GET()
                .build();
        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(this::checkStatusAsync);
    }

    public CompletableFuture<String> deleteItemAsync(String cacheId, String key) {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId + "/" + key))
                .DELETE()
                .build();
        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(this::checkStatusAsync);
    }
    
    public CompletableFuture<String> deleteCacheAsync(String cacheId) {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/" + cacheId))
                .DELETE()
                .build();
        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(this::checkStatusAsync);
    }

    // --- WebSocket ---
    
    public CompletableFuture<WebSocket> subscribe(String cacheId, WebSocket.Listener listener) {
        return httpClient.newWebSocketBuilder()
                .buildAsync(URI.create(wsUrl + "/api/ws/v1/cache/" + cacheId), listener);
    }

    public EmbeddedAeronCache getCache(String cacheId) {
        return new EmbeddedAeronCache(this, cacheId);
    }


    private void checkStatus(HttpResponse<String> response) {
        if (response.statusCode() >= 400) {
            throw new RuntimeException("Http Error: " + response.statusCode() + " Body: " + response.body());
        }
    }

    private String checkStatusAsync(HttpResponse<String> response) {
        if (response.statusCode() >= 400) {
            throw new RuntimeException("Http Error: " + response.statusCode() + " Body: " + response.body());
        }
        return response.body();
    }
}
