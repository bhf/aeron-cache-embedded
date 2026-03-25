package com.aeron.cache.sample;

import com.aeron.cache.client.AeronCacheClient;
import com.aeron.cache.client.*;
import com.aeron.cache.models.*;

import java.util.concurrent.CompletableFuture;

public class AsyncSample {
    public static void main(String[] args) {
        String baseUrl = "http://localhost:7070";

        System.out.println("Starting Async Sample against " + baseUrl);

        var wsUrl = "http://localhost:7071";
        AeronCacheClient client = new AeronCacheClient(baseUrl, wsUrl);
        try {
            var response = client.createCache("async-test-cache");
            System.out.println("Created cache: " + response.getCacheId());
        } catch (Exception e) {
            e.printStackTrace();
        }
        EmbeddedAeronCache cache = new EmbeddedAeronCache(client, "async-test-cache");

        // Put a value asynchronously
        System.out.println("Putting key 'async-key' -> 'async-value' asynchronously");
        CompletableFuture<PutItemResponse> future = cache.putAsync("async-key", "async-value");

        future.thenAccept(response -> {
            System.out.println("Put operation completed with status: " + response.getStatus());

            // Allow some time for propagation back to local cache
            try {
                Thread.sleep(100);
            } catch (InterruptedException e) {
            }

            try {
                var getResponse = cache.get("async-key");
                System.out.println("Read key 'async-key': " + getResponse.getValue());
            } catch (Exception e) {
                throw new RuntimeException(e);
            }

        }).join(); // Wait for completion for the sample

    }
}
