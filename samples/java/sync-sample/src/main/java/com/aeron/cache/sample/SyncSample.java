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

        // --- Inspection & management operations ---
        try {
            System.out.println("Patching 'doc' (deep-merge)");
            client.putItem("sync-test-cache", "doc", "{\"a\":1}");
            var patchResp = client.patchItem("sync-test-cache", "doc", "{\"b\":2}");
            System.out.println("Patch status: " + patchResp.getOperationStatus());
            System.out.println("Doc after patch: " + client.getItem("sync-test-cache", "doc").getValue());

            System.out.println("Listing all caches:");
            for (var details : client.getCaches()) {
                System.out.println("  - " + details.getCacheId() + " (" + details.getItemCount() + " items)");
            }

            var stats = client.getStats();
            System.out.println("Cache stats: caches=" + stats.getTotalCachesCount()
                    + " items=" + stats.getTotalItemsCount()
                    + " ops=" + stats.getTotalOpsCount()
                    + " errors=" + stats.getErrorCount());

            // A timed entry schedules a pending TTL removal timer; getTimers lists all pending timers
            // across both caches and counter caches, each tagged CACHE or COUNTER.
            client.putTimedItem("sync-test-cache", "expiring", "gone-soon", 600_000);
            System.out.println("Listing all pending TTL timers:");
            for (var timer : client.getTimers().getTimers()) {
                System.out.println("  - [" + timer.getTimerType() + "] " + timer.getCacheId()
                        + "/" + timer.getKey() + " fires at " + timer.getDeadline());
            }

            System.out.println("Listing all counters in 'sync-counter-cache':");
            for (var item : client.getCounterItems("sync-counter-cache").getItems()) {
                System.out.println("  - " + item.getKey() + " = " + item.getValue());
            }

            var counterStats = client.getCounterStats();
            System.out.println("Counter stats: caches=" + counterStats.getTotalCachesCount()
                    + " items=" + counterStats.getTotalItemsCount());
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    }
}
