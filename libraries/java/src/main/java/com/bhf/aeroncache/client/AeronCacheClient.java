package com.bhf.aeroncache.client;

import com.bhf.aeroncache.models.*;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.net.http.WebSocket;
import java.util.concurrent.CompletableFuture;
import java.util.function.Consumer;

public class AeronCacheClient implements CacheTransport {
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

    public PutItemResponse putTimedItem(String cacheId, String key, String value, long ttl) throws Exception {
        String json = String.format("{\"key\":\"%s\",\"value\":\"%s\",\"ttl\":%d}", key, value, ttl);
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/timed/" + cacheId))
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

    public BulkCacheOpsResponse bulkOps(BulkCacheOpsRequest bulkOpsRequest) throws Exception {
        String json = objectMapper.writeValueAsString(bulkOpsRequest);
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/bulkops"))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), BulkCacheOpsResponse.class);
    }

    // --- Counter Operations (Sync) ---

    public CreateResponse createCounterCache(String cacheId) throws Exception {
        String json = "{\"cacheId\":\"" + cacheId + "\"}";
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/counters/"))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), CreateResponse.class);
    }

    public PutItemResponse putCounter(String cacheId, String key, long value) throws Exception {
        String json = String.format("{\"key\":\"%s\",\"value\":%d}", key, value);
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/counters/" + cacheId))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), PutItemResponse.class);
    }

    public PutItemResponse putTimedCounter(String cacheId, String key, long value, long ttl) throws Exception {
        String json = String.format("{\"key\":\"%s\",\"value\":%d,\"ttl\":%d}", key, value, ttl);
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/counters/timed/" + cacheId))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), PutItemResponse.class);
    }

    public CounterResponse getCounter(String cacheId, String key) throws Exception {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/counters/" + cacheId + "/" + key))
                .GET()
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), CounterResponse.class);
    }

    public DeleteItemResponse deleteCounter(String cacheId, String key) throws Exception {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/counters/" + cacheId + "/" + key))
                .DELETE()
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), DeleteItemResponse.class);
    }

    public DeleteCacheResponse deleteCounterCache(String cacheId) throws Exception {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/counters/" + cacheId))
                .DELETE()
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), DeleteCacheResponse.class);
    }

    public CounterResponse incrementCounter(String cacheId, String key, long amount) throws Exception {
        return counterAmountOp("increment", cacheId, key, amount);
    }

    public CounterResponse decrementCounter(String cacheId, String key, long amount) throws Exception {
        return counterAmountOp("decrement", cacheId, key, amount);
    }

    private CounterResponse counterAmountOp(String op, String cacheId, String key, long amount) throws Exception {
        String json = String.format("{\"key\":\"%s\",\"amount\":%d}", key, amount);
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/counters/" + op + "/" + cacheId))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), CounterResponse.class);
    }

    public CounterResponse setCounter(String cacheId, String key, long value) throws Exception {
        String json = String.format("{\"key\":\"%s\",\"value\":%d}", key, value);
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/counters/set/" + cacheId))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .build();
        HttpResponse<String> response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());
        checkStatus(response);
        return objectMapper.readValue(response.body(), CounterResponse.class);
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

    public CompletableFuture<PutItemResponse> putTimedItemAsync(String cacheId, String key, String value, long ttl) {
        String json = String.format("{\"key\":\"%s\",\"value\":\"%s\",\"ttl\":%d}", key, value, ttl);
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/api/v1/cache/timed/" + cacheId))
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

    public CompletableFuture<BulkCacheOpsResponse> bulkOpsAsync(BulkCacheOpsRequest bulkOpsRequest) {
        try {
            String json = objectMapper.writeValueAsString(bulkOpsRequest);
            HttpRequest request = HttpRequest.newBuilder()
                    .uri(URI.create(baseUrl + "/api/v1/cache/bulkops"))
                    .header("Content-Type", "application/json")
                    .POST(HttpRequest.BodyPublishers.ofString(json))
                    .build();
            return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                    .thenApply(resp -> {
                        checkStatusAsync(resp);
                        try {
                            return objectMapper.readValue(resp.body(), BulkCacheOpsResponse.class);
                        } catch (Exception e) {
                            throw new RuntimeException(e);
                        }
                    });
        } catch (Exception e) {
            return CompletableFuture.failedFuture(e);
        }
    }

    // --- Counter Operations (Async) ---

    public CompletableFuture<CreateResponse> createCounterCacheAsync(String cacheId) {
        String json = "{\"cacheId\":\"" + cacheId + "\"}";
        return sendAsync(URI.create(baseUrl + "/api/v1/counters/"), "POST", json, CreateResponse.class);
    }

    public CompletableFuture<PutItemResponse> putCounterAsync(String cacheId, String key, long value) {
        String json = String.format("{\"key\":\"%s\",\"value\":%d}", key, value);
        return sendAsync(URI.create(baseUrl + "/api/v1/counters/" + cacheId), "POST", json, PutItemResponse.class);
    }

    public CompletableFuture<PutItemResponse> putTimedCounterAsync(String cacheId, String key, long value, long ttl) {
        String json = String.format("{\"key\":\"%s\",\"value\":%d,\"ttl\":%d}", key, value, ttl);
        return sendAsync(URI.create(baseUrl + "/api/v1/counters/timed/" + cacheId), "POST", json, PutItemResponse.class);
    }

    public CompletableFuture<CounterResponse> getCounterAsync(String cacheId, String key) {
        return sendAsync(URI.create(baseUrl + "/api/v1/counters/" + cacheId + "/" + key), "GET", null, CounterResponse.class);
    }

    public CompletableFuture<DeleteItemResponse> deleteCounterAsync(String cacheId, String key) {
        return sendAsync(URI.create(baseUrl + "/api/v1/counters/" + cacheId + "/" + key), "DELETE", null, DeleteItemResponse.class);
    }

    public CompletableFuture<DeleteCacheResponse> deleteCounterCacheAsync(String cacheId) {
        return sendAsync(URI.create(baseUrl + "/api/v1/counters/" + cacheId), "DELETE", null, DeleteCacheResponse.class);
    }

    public CompletableFuture<CounterResponse> incrementCounterAsync(String cacheId, String key, long amount) {
        String json = String.format("{\"key\":\"%s\",\"amount\":%d}", key, amount);
        return sendAsync(URI.create(baseUrl + "/api/v1/counters/increment/" + cacheId), "POST", json, CounterResponse.class);
    }

    public CompletableFuture<CounterResponse> decrementCounterAsync(String cacheId, String key, long amount) {
        String json = String.format("{\"key\":\"%s\",\"amount\":%d}", key, amount);
        return sendAsync(URI.create(baseUrl + "/api/v1/counters/decrement/" + cacheId), "POST", json, CounterResponse.class);
    }

    public CompletableFuture<CounterResponse> setCounterAsync(String cacheId, String key, long value) {
        String json = String.format("{\"key\":\"%s\",\"value\":%d}", key, value);
        return sendAsync(URI.create(baseUrl + "/api/v1/counters/set/" + cacheId), "POST", json, CounterResponse.class);
    }

    private <T> CompletableFuture<T> sendAsync(URI uri, String method, String body, Class<T> type) {
        HttpRequest.Builder builder = HttpRequest.newBuilder().uri(uri);
        if ("POST".equals(method)) {
            builder.header("Content-Type", "application/json")
                    .POST(HttpRequest.BodyPublishers.ofString(body == null ? "" : body));
        } else if ("DELETE".equals(method)) {
            builder.DELETE();
        } else {
            builder.GET();
        }
        return httpClient.sendAsync(builder.build(), HttpResponse.BodyHandlers.ofString())
                .thenApply(resp -> {
                    checkStatusAsync(resp);
                    try {
                        return objectMapper.readValue(resp.body(), type);
                    } catch (Exception e) {
                        throw new RuntimeException(e);
                    }
                });
    }

    // --- WebSocket ---

    public ReconnectingWebSocket subscribe(String cacheId, WebSocket.Listener listener) {
        return subscribe(cacheId, false, listener);
    }

    public ReconnectingWebSocket subscribeCounter(String cacheId, WebSocket.Listener listener) {
        return subscribeCounter(cacheId, false, listener);
    }

    public ReconnectingWebSocket subscribeCounter(String cacheIds, boolean hydrate, WebSocket.Listener listener) {
        String finalWsUrl = wsUrl;
        String prefix = hydrate ? "/api/ws/v1/counter/hydrate" : "/api/ws/v1/counter";

        // Handle plural if comma-separated
        if (cacheIds.contains(",")) {
            prefix = hydrate ? "/api/ws/v1/counters/hydrate" : "/api/ws/v1/counters";
        }

        if (finalWsUrl.endsWith("/")) {
            finalWsUrl = finalWsUrl.substring(0, finalWsUrl.length() - 1);
        }

        URI uri = URI.create(finalWsUrl + prefix + "/" + cacheIds);
        return new ReconnectingWebSocket(httpClient, uri, listener);
    }

    public EmbeddedCounterCache getCounterCache(String cacheId) {
        return new EmbeddedCounterCache(this, cacheId);
    }

    public ReconnectingWebSocket subscribe(String cacheIds, boolean hydrate, WebSocket.Listener listener) {
        String finalWsUrl = wsUrl;
        String prefix = hydrate ? "/api/ws/v1/cache/hydrate" : "/api/ws/v1/cache";
        
        // Handle plural if comma-separated
        if (cacheIds.contains(",")) {
            prefix = hydrate ? "/api/ws/v1/caches/hydrate" : "/api/ws/v1/caches";
        }

        if (finalWsUrl.endsWith("/")) {
            finalWsUrl = finalWsUrl.substring(0, finalWsUrl.length() - 1);
        }
        
        URI uri = URI.create(finalWsUrl + prefix + "/" + cacheIds);
        return new ReconnectingWebSocket(httpClient, uri, listener);
    }

    public EmbeddedAeronCache getCache(String cacheId) {
        return new EmbeddedAeronCache(this, cacheId);
    }

    // --- Transport-neutral subscriptions (CacheTransport) ---

    @Override
    public AutoCloseable subscribeCacheUpdates(String cacheId, boolean hydrate, Consumer<CacheUpdateEvent> listener) {
        AeronCacheSubscriber subscriber = new AeronCacheSubscriber() {
            @Override
            public void onAfterUpdate(CacheUpdateEvent event) {
                if (listener != null) {
                    listener.accept(event);
                }
            }
        };
        ReconnectingWebSocket ws = subscribe(cacheId, hydrate, subscriber);
        return ws::close;
    }

    @Override
    public AutoCloseable subscribeCounterUpdates(String cacheId, boolean hydrate, Consumer<CounterUpdateEvent> listener) {
        CounterCacheSubscriber subscriber = new CounterCacheSubscriber() {
            @Override
            public void onAfterUpdate(CounterUpdateEvent event) {
                if (listener != null) {
                    listener.accept(event);
                }
            }
        };
        ReconnectingWebSocket ws = subscribeCounter(cacheId, hydrate, subscriber);
        return ws::close;
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
