package com.aeron.cache.sample;

import com.aeron.cache.embedded.EmbeddedAeronCache;
import java.util.Optional;

public class SyncSample {
    public static void main(String[] args) {
        String baseUrl = "http://localhost:8080";
        if (args.length > 0) {
            baseUrl = args[0];
        }

        System.out.println("Starting Sync Sample against " + baseUrl);
        
        // Initialize the embedded cache
        try (EmbeddedAeronCache cache = new EmbeddedAeronCache(baseUrl)) {
            
            // Put a value
            System.out.println("Putting key 'sync-key' -> 'sync-value'");
            cache.put("sync-key", "sync-value");
            
            // Get a value (this reads from the local replicated map, so we might need a small wait if we just wrote it
            // and expect it to come back via WebSocket replication if the cache is fully async under the hood,
            // but the put sends a REST request. The local map update happens via WebSocket.)
            
            // For the sake of the sample, we can sleep briefly to allow the WebSocket update to arrive
            Thread.sleep(100); 
            
            Optional<String> value = cache.get("sync-key");
            if (value.isPresent()) {
                 System.out.println("Read back key 'sync-key': " + value.get());
            } else {
                 System.out.println("Key 'sync-key' not found in local cache yet.");
            }

        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
