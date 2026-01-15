package com.aeron.cache.sample;

import com.aeron.cache.client.AeronCacheClient;
import com.aeron.cache.client.EmbeddedAeronCache;

import java.util.concurrent.CompletableFuture;

public class AsyncSample {
    public static void main(String[] args) {
        String baseUrl = "http://localhost:7070";

        System.out.println("Starting Async Sample against " + baseUrl);

        var wsUrl = "http://localhost:7071";
        AeronCacheClient client = new AeronCacheClient(baseUrl, wsUrl);
        try {
            client.createCache("async-test-cache");
            System.out.println("Created cache 'async-test-cache'");
        } catch (Exception e) {
            e.printStackTrace();
        }
        EmbeddedAeronCache cache = new EmbeddedAeronCache(client, "async-test-cache");

        // Put a value asynchronously
        System.out.println("Putting key 'async-key' -> 'async-value' asynchronously");
        CompletableFuture<String> future = cache.putAsync("async-key", "async-value");

        future.thenRun(() -> {
            System.out.println("Put operation completed.");

            // Allow some time for propagation back to local cache
            try {
                Thread.sleep(100);
            } catch (InterruptedException e) {
            }

            String val = null;
            try {
                val = cache.get("async-key");
                System.out.println("Read key 'async-key': " + val);
            } catch (Exception e) {
                throw new RuntimeException(e);
            }

        }).join(); // Wait for completion for the sample

    }
}
