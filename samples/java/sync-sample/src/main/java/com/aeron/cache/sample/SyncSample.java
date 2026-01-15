package com.aeron.cache.sample;

import com.aeron.cache.client.AeronCacheClient;
import com.aeron.cache.client.EmbeddedAeronCache;

public class SyncSample {
    public static void main(String[] args) {
        String baseUrl = "http://localhost:7070";

        System.out.println("Starting Sync Sample against " + baseUrl);

        // Initialize the embedded cache
        var wsUrl = "http://localhost:7071";
        AeronCacheClient client = new AeronCacheClient(baseUrl, wsUrl);
        try {
            client.createCache("sync-test-cache");
            System.out.println("Created cache 'sync-test-cache'");
        } catch (Exception e) {
            e.printStackTrace();
        }
        EmbeddedAeronCache cache = new EmbeddedAeronCache(client, "sync-test-cache");

        // Put a value
        System.out.println("Putting key 'sync-key' -> 'sync-value'");
        try {
            cache.put("sync-key", "sync-value");
        } catch (Exception e) {
            throw new RuntimeException(e);
        }

        try {
            Thread.sleep(1000);
        } catch (InterruptedException e) {
            throw new RuntimeException(e);
        }

        try {
            var value = cache.get("sync-key");
            System.out.println("Retrieved key 'sync-key' -> '" + value + "'");
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    }
}
