package com.aeron.cache.sample;

import com.bhf.aeroncache.client.AeronCacheClient;
import com.bhf.aeroncache.client.AeronCacheSubscriber;
import com.bhf.aeroncache.client.EmbeddedObjectCache;
import com.bhf.aeroncache.models.CacheUpdateEvent;

import java.net.http.WebSocket;
import java.util.List;

/**
 * Demonstrates server-side synthetic PATCH streaming.
 *
 * <p>The sample subscribes in {@code patch} mode and then writes evolving <em>full</em> JSON values with
 * ordinary {@code putItem} calls (never {@code patchItem}). As long as a patch-mode subscriber is listening,
 * the cache service turns those puts into patch traffic:
 * <ul>
 *   <li>the first put of a key arrives as a full {@code ADD_ITEM} baseline,</li>
 *   <li>each later overwrite arrives as a {@code PATCH_ITEM} delta carrying only the changed fields,</li>
 *   <li>an identical (no-op) put is suppressed (no event), and</li>
 *   <li>a non-JSON value falls back to a full {@code ADD_ITEM}.</li>
 * </ul>
 * The {@link EmbeddedObjectCache} deep-merges each delta (RFC 7386), so {@code getLocal} always holds the
 * full, reconstructed object with untouched (including nested) fields preserved.
 */
public class PatchStreamSample {
    public static void main(String[] args) throws Exception {
        String baseUrl = args.length > 0 ? args[0] : "http://localhost:7070";
        String wsUrl = args.length > 1 ? args[1] : "ws://localhost:7071";

        System.out.println("Starting Patch Stream Sample against " + baseUrl + ", ws=" + wsUrl);

        AeronCacheClient client = new AeronCacheClient(baseUrl, wsUrl);
        String cacheId = "patch-stream-sample-cache";
        try {
            client.createCache(cacheId);
            System.out.println("Created cache: " + cacheId);
        } catch (Exception e) {
            // Probably already exists.
        }

        EmbeddedObjectCache objects = client.getObjectCache(cacheId);

        // Start from a clean slate so the first put is a fresh insert (an ADD_ITEM baseline) rather than an
        // overwrite of a value left over from a previous run (which would arrive as a PATCH_ITEM delta).
        try {
            objects.remove("doc");
        } catch (Exception e) {
            // Key may not exist.
        }

        // Subscribe in PATCH mode BEFORE the key exists, so the first put arrives as a full ADD_ITEM
        // baseline and every later put as a PATCH_ITEM delta the object cache deep-merges locally.
        objects.subscribe(new AeronCacheSubscriber() {
            @Override
            public void onOpen(WebSocket webSocket) {
                super.onOpen(webSocket);
                System.out.println("[Java] patch-mode subscription opened");
            }

            @Override
            public void onAfterUpdate(CacheUpdateEvent event) {
                if (!"doc".equals(event.getItemKey())) {
                    return;
                }
                System.out.println("[Java] <- " + event.getEventType() + " delta=" + event.getItemValue()
                        + "  reconstructed=" + objects.getLocal("doc"));
            }
        }, false, null, "patch");

        // Evolving FULL values written with regular puts; the server computes the diff against the prior
        // value. Note the deliberate no-op (same value twice -> suppressed) and the field removal (status
        // dropped -> delta carries status:null, which deletes the field in the mirror).
        List<String> states = List.of(
                "{\"user\":{\"name\":\"ada\",\"age\":36},\"status\":\"active\"}",
                "{\"user\":{\"name\":\"ada\",\"age\":37},\"status\":\"active\"}", // delta: user.age -> 37
                "{\"user\":{\"name\":\"ada\",\"age\":37},\"status\":\"away\"}",   // delta: status -> away
                "{\"user\":{\"name\":\"ada\",\"age\":37},\"status\":\"away\"}",   // identical -> suppressed
                "{\"user\":{\"name\":\"ada\",\"age\":37}}"                          // delta: status -> null (removed)
        );

        Thread.sleep(800); // let the subscription register before the first put
        int i = 0;
        while (true) {
            String state = states.get(i % states.size());
            System.out.println("[Java] -> put doc = " + state);
            client.putItem(cacheId, "doc", state);
            i++;
            Thread.sleep(2500);
        }
    }
}
