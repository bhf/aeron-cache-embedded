package com.aeron.cache.sample;

import com.bhf.aeroncache.client.bidi.AeronBidiClient;
import com.bhf.aeroncache.client.bidi.BidiSubscription;
import com.bhf.aeroncache.models.BulkCacheOpsRequest;
import com.bhf.aeroncache.models.BulkOperationType;
import com.bhf.aeroncache.models.CacheItem;
import com.bhf.aeroncache.models.CacheOperationRequest;
import com.bhf.aeroncache.models.CacheOperationResponse;
import com.bhf.aeroncache.models.StatEntry;
import com.bhf.aeroncache.models.TimerInfo;

import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

/**
 * Demonstrates the bidirectional WebSocket transport — the JSON/WebSocket analogue of the Aeron
 * gateway client, carried over a single persistent WebSocket connection ({@code /api/ws/v1/bidi}).
 * <p>
 * Point it at the backend's bidi WebSocket endpoint with {@code args[0]} (default
 * {@code ws://localhost:7071}). The backend must be running with the WebSocket transport enabled.
 */
public class BidiSample {

    public static void main(String[] args) throws Exception {
        final String wsUrl = args.length > 0 ? args[0] : "ws://localhost:7071";
        System.out.println("Starting Bidi Sample against " + wsUrl);

        try (AeronBidiClient client = new AeronBidiClient(wsUrl)) {
            client.connect();
            System.out.println("Connected to bidi WebSocket endpoint.");

            // --- Cache operations ---
            final String cacheId = "bidi-sample-cache";
            System.out.println();
            System.out.println("--- Cache ---");
            System.out.println("Created cache: " + client.createCache(cacheId).getCacheId());
            System.out.println("Put bidi-key -> bidi-value: "
                    + client.putItem(cacheId, "bidi-key", "bidi-value").getOperationStatus());
            System.out.println("Get bidi-key -> " + client.getItem(cacheId, "bidi-key").getValue());

            // --- Patch (JSON merge) ---
            // Put a base document, then patch it: the server merges the patch into the stored value.
            System.out.println();
            System.out.println("--- Patch ---");
            client.putItem(cacheId, "doc", "{\"a\":1}");
            System.out.println("Put doc -> {\"a\":1}");
            System.out.println("Patch doc <- {\"b\":2}: "
                    + client.patchItem(cacheId, "doc", "{\"b\":2}").getOperationStatus());
            System.out.println("Merged doc -> " + client.getItem(cacheId, "doc").getValue());

            // --- Counters ---
            System.out.println();
            System.out.println("--- Counters ---");
            final String counterCache = "bidi-sample-counters";
            System.out.println("Created counter cache: " + client.createCounterCache(counterCache).getCacheId());
            System.out.println("Put hits = 10: " + client.putCounter(counterCache, "hits", 10).getOperationStatus());
            System.out.println("increment hits +5 -> " + client.incrementCounter(counterCache, "hits", 5).getValue());
            System.out.println("Get hits -> " + client.getCounter(counterCache, "hits").getValue());

            // --- Bulk read + stats ---
            System.out.println();
            System.out.println("--- Entries & stats ---");
            System.out.println("Cache items:");
            for (CacheItem item : client.getCacheItems(cacheId).getItems()) {
                System.out.println("  " + item.getKey() + " = " + item.getValue());
            }
            System.out.println("Cache stats:");
            for (StatEntry stat : client.getStats()) {
                System.out.println("  " + stat.getCacheId() + " size=" + stat.getSize()
                        + " added=" + stat.getAddedCount() + " removed=" + stat.getRemovedCount());
            }

            // --- Bulk operations ---
            // A single `bulk` frame carries a batch of operations (regular-cache and counter ops may be
            // mixed); the server streams back per-operation results, each echoing its own requestId.
            System.out.println();
            System.out.println("--- Bulk operations ---");
            final BulkCacheOpsRequest bulk = BulkCacheOpsRequest.builder()
                    .requestId("bidi-bulk-1")
                    .addOperation(CacheOperationRequest.builder()
                            .operationType(BulkOperationType.ADD_ITEM)
                            .requestId("op-1").cacheId(cacheId).key("bk1").value("bv1").build())
                    .addOperation(CacheOperationRequest.builder()
                            .operationType(BulkOperationType.ADD_ITEM)
                            .requestId("op-2").cacheId(cacheId).key("bk2").value("bv2").build())
                    .addOperation(CacheOperationRequest.builder()
                            .operationType(BulkOperationType.GET_ITEM)
                            .requestId("op-3").cacheId(cacheId).key("bk1").build())
                    .build();
            for (CacheOperationResponse op : client.bulkOps(bulk).getOperationResponses()) {
                System.out.println("  " + op.getRequestId() + " -> " + op.getStatus()
                        + (op.getValue() != null ? " (" + op.getValue() + ")" : ""));
            }

            // --- Timers ---
            // A timed entry schedules a pending TTL removal timer; getTimers lists all pending timers
            // (cache + counter), each tagged with its type.
            System.out.println();
            System.out.println("--- Timers ---");
            client.putTimedItem(cacheId, "expiring", "gone-soon", 600_000);
            for (TimerInfo timer : client.getTimers()) {
                System.out.println("  [" + timer.getTimerType() + "] " + timer.getCacheId()
                        + "/" + timer.getKey() + " fires at " + timer.getDeadline());
            }

            // --- Live subscription ---
            // Subscribe over the same connection, then write a key and observe the streamed update.
            System.out.println();
            System.out.println("--- Live subscription ---");
            final CountDownLatch received = new CountDownLatch(1);
            try (BidiSubscription subscription = client.subscribe(cacheId, event -> {
                System.out.println("  [update] " + event.getEventType() + " "
                        + event.getItemKey() + "=" + event.getItemValue());
                received.countDown();
            })) {
                // Let the subscription establish before writing, so we observe our own update.
                Thread.sleep(500);
                client.putItem(cacheId, "streamed-key", "streamed-value");
                if (!received.await(5, TimeUnit.SECONDS)) {
                    System.out.println("  (no streamed update received within 5s)");
                }
            }
            System.out.println("Subscription closed.");

            // --- Cleanup ---
            System.out.println();
            client.deleteCache(cacheId);
            client.deleteCounterCache(counterCache);
            System.out.println("Deleted caches. Done.");
        } catch (Exception e) {
            System.err.println("Bidi sample failed — is the backend running with the WebSocket "
                    + "transport reachable at " + wsUrl + "?");
            System.err.println("  cause: " + e.getMessage());
        }
    }
}
