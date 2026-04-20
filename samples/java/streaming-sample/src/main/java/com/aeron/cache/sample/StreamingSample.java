package com.aeron.cache.sample;


import com.aeron.cache.client.AeronCacheClient;
import com.aeron.cache.client.AeronCacheSubscriber;
import com.aeron.cache.client.EmbeddedAeronCache;
import com.aeron.cache.models.CacheUpdateEvent;
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

        // Subscribe explicitly using AeronCacheSubscriber abstraction
        cache.subscribe(new AeronCacheSubscriber() {
            @Override
            public void onOpen(WebSocket webSocket) {
                super.onOpen(webSocket);
                System.out.println("[Java] WebSocket subscription opened");
            }

            @Override
            public void onAfterUpdate(CacheUpdateEvent event) {
                System.out.println("[Java] Received update: " + event.getEventType() + " for " + event.getItemKey());
            }
        });

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
