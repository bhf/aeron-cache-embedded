package com.aeron.cache.sample;

import com.bhf.aeroncache.client.AeronCacheClient;
import com.bhf.aeroncache.client.EmbeddedAeronCache;
import com.bhf.aeroncache.client.EmbeddedCounterCache;

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

        System.out.println("Putting key 'timed-key' -> 'timed-value' with 5000ms TTL");
        try {
            var response = cache.putTimed("timed-key", "timed-value", 5000L);
            System.out.println("Timed Put response: " + response.getOperationStatus());
        } catch (Exception e) {
            throw new RuntimeException(e);
        }

        try {
            var response = cache.get("timed-key");
            System.out.println("Retrieved key 'timed-key' -> '" + response.getValue() + "'");
        } catch (Exception e) {
            throw new RuntimeException(e);
        }

        // --- Counter operations ---
        try {
            var response = client.createCounterCache("sync-counter-cache");
            System.out.println("Created counter cache: " + response.getCacheId());
        } catch (Exception e) {
            // Probably already exists
        }
        EmbeddedCounterCache counters = new EmbeddedCounterCache(client, "sync-counter-cache");

        try {
            System.out.println("Putting counter 'requests' -> 10");
            counters.put("requests", 10);
            System.out.println("Incremented 'requests' by 5 -> " + counters.increment("requests", 5).getValue());
            System.out.println("Decremented 'requests' by 3 -> " + counters.decrement("requests", 3).getValue());
            System.out.println("Set 'requests' -> " + counters.set("requests", 100).getValue());
            System.out.println("Read counter 'requests': " + counters.get("requests").getValue());
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    }
}
