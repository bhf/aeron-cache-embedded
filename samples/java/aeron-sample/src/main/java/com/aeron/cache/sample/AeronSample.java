package com.aeron.cache.sample;

import com.bhf.aeroncache.client.EmbeddedAeronCache;
import com.bhf.aeroncache.client.gateway.AeronGatewayClient;
import com.bhf.aeroncache.client.gateway.GatewaySubscription;
import io.aeron.driver.MediaDriver;
import io.aeron.driver.ThreadingMode;

import java.util.concurrent.TimeUnit;

/**
 * Demonstrates the Aeron gateway transport — the alternative to the HTTP+WS client.
 * <p>
 * Launches an embedded media driver and talks to a gateway over UDP. Enable the gateway on the
 * backend with {@code -Daeron.transport.gateway.enabled=true}. Override the host with
 * {@code -Daeron.gateway.host=<host>} (default {@code 127.0.0.1}).
 */
public class AeronSample {

    public static void main(String[] args) throws Exception {
        final String host = System.getProperty("aeron.gateway.host", "127.0.0.1");
        System.out.println("Starting Aeron Sample against gateway " + host);

        // Embedded media driver so the sample is self-contained; the gateway is reached over UDP.
        try (MediaDriver driver = MediaDriver.launchEmbedded(new MediaDriver.Context()
                .threadingMode(ThreadingMode.SHARED)
                .dirDeleteOnStart(true)
                .dirDeleteOnShutdown(true));
             AeronGatewayClient client = AeronGatewayClient.connect(driver.aeronDirectoryName(), host)) {

            if (!client.awaitConnected(10, TimeUnit.SECONDS)) {
                System.err.println("Could not connect to the gateway — is it enabled and reachable?");
                return;
            }
            System.out.println("Connected to gateway.");

            // --- Cache operations ---
            final String cacheId = "aeron-sample-cache";
            System.out.println("Created cache: " + client.createCache(cacheId).getCacheId());
            System.out.println("Put aeron-key -> aeron-value: " + client.putItem(cacheId, "aeron-key", "aeron-value").getOperationStatus());
            System.out.println("Get aeron-key -> " + client.getItem(cacheId, "aeron-key").getValue());

            // --- Streaming subscription ---
            try (GatewaySubscription subscription = client.subscribe(cacheId, event ->
                    System.out.println("  [update] " + event.getEventType() + " " + event.getItemKey() + "=" + event.getItemValue()))) {
                Thread.sleep(500);
                client.putItem(cacheId, "streamed-key", "streamed-value");
                Thread.sleep(1000);
            }

            // --- Embedded cache over the gateway ---
            // An EmbeddedAeronCache keeps a local map in sync with the cache by subscribing to streaming
            // updates over the same Aeron connection. Writes go to the gateway; reads are served locally
            // with no network round-trip. The API is transport-neutral: identical over HTTP+WS or Aeron.
            System.out.println();
            System.out.println("--- Embedded cache ---");
            final String embeddedCacheId = "aeron-sample-embedded";
            client.createCache(embeddedCacheId);
            final EmbeddedAeronCache embedded = client.getCache(embeddedCacheId);

            // subscribe() wires the local mirror and forwards each event to the listener below.
            try (AutoCloseable subscription = embedded.subscribe(event ->
                    System.out.println("  [mirror] " + event.getEventType() + " " + event.getItemKey() + "=" + event.getItemValue()))) {

                // Let the subscription establish before writing, so we observe our own updates.
                Thread.sleep(500);

                // Write through the embedded cache (goes to the gateway)...
                embedded.put("user:1", "Ada");
                embedded.put("user:2", "Alan");

                // ...and wait for the streamed updates to populate the local mirror.
                Thread.sleep(1000);

                // Local reads — served from the in-memory mirror, no network call.
                System.out.println("Local read user:1 -> " + embedded.getLocal("user:1"));
                System.out.println("Local read user:2 -> " + embedded.getLocal("user:2"));
                System.out.println("Local mirror snapshot -> " + embedded.getLocalCache());

                // Removals are mirrored too.
                embedded.remove("user:1");
                Thread.sleep(1000);
                System.out.println("After remove, local read user:1 -> " + embedded.getLocal("user:1"));
                System.out.println("Local mirror snapshot -> " + embedded.getLocalCache());
            }

            // --- Counter operations ---
            System.out.println();
            System.out.println("--- Counters ---");
            final String counterCache = "aeron-sample-counters";
            System.out.println("Created counter cache: " + client.createCounterCache(counterCache).getCacheId());
            client.putCounter(counterCache, "hits", 10);
            System.out.println("increment hits +5 -> " + client.incrementCounter(counterCache, "hits", 5).getValue());
            System.out.println("decrement hits -3 -> " + client.decrementCounter(counterCache, "hits", 3).getValue());
            System.out.println("set hits = 100 -> " + client.setCounter(counterCache, "hits", 100).getValue());

            client.deleteCache(cacheId);
            client.deleteCache(embeddedCacheId);
            client.deleteCounterCache(counterCache);
            System.out.println("Done.");
        }
    }
}
