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
            var response = client.createCache("sync-test-cache");
            System.out.println("Created cache: " + response.getCacheId());
        } catch (Exception e) {
            e.printStackTrace();
        }
        EmbeddedAeronCache cache = new EmbeddedAeronCache(client, "sync-test-cache");

        // Put a value
        System.out.println("Putting key 'sync-key' -> 'sync-value'");
        try {
            var response = cache.put("sync-key", "sync-value");
            System.out.println("Put response: " + response.getStatus());
        } catch (Exception e) {
            throw new RuntimeException(e);
        }

        try {
            Thread.sleep(1000);
        } catch (InterruptedException e) {
            throw new RuntimeException(e);
        }

        try {
            var response = cache.get("sync-key");
            System.out.println("Retrieved key 'sync-key' -> '" + response.getValue() + "'");
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    }
}
