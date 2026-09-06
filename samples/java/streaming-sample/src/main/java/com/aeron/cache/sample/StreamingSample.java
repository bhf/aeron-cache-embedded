package com.aeron.cache.sample;


import com.bhf.aeroncache.client.AeronCacheClient;
import com.bhf.aeroncache.client.AeronCacheSubscriber;
import com.bhf.aeroncache.client.CounterCacheSubscriber;
import com.bhf.aeroncache.client.EmbeddedAeronCache;
import com.bhf.aeroncache.client.EmbeddedCounterCache;
import com.bhf.aeroncache.models.CacheUpdateEvent;
import com.bhf.aeroncache.models.CounterUpdateEvent;
import java.net.http.WebSocket;

public class StreamingSample {
    public static void main(String[] args) {
        String baseUrl = "http://localhost:7070";

        System.out.println("Starting Streaming Sample against " + baseUrl);

        var wsUrl = "ws://localhost:7071";
        AeronCacheClient client = new AeronCacheClient(baseUrl, wsUrl);
        try {
            var response = client.createCache("streaming-sample-cache");
            System.out.println("Created cache: " + response.getCacheId());
        } catch (Exception e) {
            // Probably already exists
        }
        EmbeddedAeronCache cache = new EmbeddedAeronCache(client, "streaming-sample-cache");

        // Subscribe explicitly using AeronCacheSubscriber abstraction with hydration
        cache.subscribe(new AeronCacheSubscriber() {
            @Override
            public void onOpen(WebSocket webSocket) {
                super.onOpen(webSocket);
                System.out.println("[Java] WebSocket subscription opened (with hydration)");
            }

            @Override
            public void onAfterUpdate(CacheUpdateEvent event) {
                System.out.println("[Java] Received update: " + event.getEventType() + " for " + event.getItemKey());
            }
        }, true);

        // --- Counter streaming (with hydration) ---
        try {
            var response = client.createCounterCache("streaming-counter-cache");
            System.out.println("Created counter cache: " + response.getCacheId());
        } catch (Exception e) {
            // Probably already exists
        }
        EmbeddedCounterCache counters = new EmbeddedCounterCache(client, "streaming-counter-cache");
        counters.subscribe(new CounterCacheSubscriber() {
            @Override
            public void onAfterUpdate(CounterUpdateEvent event) {
                System.out.println("[Java] Counter update: " + event.getEventType()
                        + " for " + event.getItemKey() + " -> " + event.getItemValue());
            }
        }, true);

        // Periodically increment a counter to generate streaming updates.
        Thread ticker = new Thread(() -> {
            while (true) {
                try {
                    counters.increment("tick", 1);
                    Thread.sleep(2000);
                } catch (Exception e) {
                    return;
                }
            }
        });
        ticker.setDaemon(true);
        ticker.start();

        System.out.println("Listening for updates on 'streaming-sample-cache'.");

        String lastValue = "";
        while (true) {
            // Poll using getLocal instead of get
            String current = cache.getLocal("streaming-key");
            
            if (current != null && !current.isEmpty() && !current.equals(lastValue)) {
                System.out.println("[Java-Poller] Observed change in 'streaming-key': " + current);
                lastValue = current;
            }

            try {
                Thread.sleep(1000);
            } catch (InterruptedException e) {
                throw new RuntimeException(e);
            }
        }
    }
}
