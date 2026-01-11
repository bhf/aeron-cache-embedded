package com.aeron.cache.sample;

import com.aeron.cache.embedded.EmbeddedAeronCache;
import java.util.concurrent.CompletableFuture;

public class AsyncSample {
    public static void main(String[] args) {
        String baseUrl = "http://localhost:8080";
        if (args.length > 0) {
            baseUrl = args[0];
        }

        System.out.println("Starting Async Sample against " + baseUrl);

        try (EmbeddedAeronCache cache = new EmbeddedAeronCache(baseUrl)) {
            
            // Put a value asynchronously
            System.out.println("Putting key 'async-key' -> 'async-value' asynchronously");
            CompletableFuture<Void> future = cache.putAsync("async-key", "async-value");
            
            future.thenRun(() -> {
                System.out.println("Put operation completed.");
                
                // Allow some time for propagation back to local cache
                try { Thread.sleep(100); } catch (InterruptedException e) {}

                String val = cache.get("async-key").orElse("not found");
                System.out.println("Read key 'async-key': " + val);

            }).join(); // Wait for completion for the sample
            
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
