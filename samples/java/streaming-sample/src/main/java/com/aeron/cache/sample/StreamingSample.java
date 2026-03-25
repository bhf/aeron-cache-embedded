package com.aeron.cache.sample;


import com.aeron.cache.client.AeronCacheClient;
import com.aeron.cache.client.EmbeddedAeronCache;
import java.net.http.WebSocket;
import java.util.concurrent.CompletionStage;

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

        // Subscribe explicitly
        cache.subscribe(new WebSocket.Listener() {
            @Override
            public void onOpen(WebSocket webSocket) {
                System.out.println("[Java] WebSocket subscription opened");
            }

            @Override
            public CompletionStage<?> onText(WebSocket webSocket, CharSequence data, boolean last) {
                System.out.println("[Java] Received update: " + data);
                return WebSocket.Listener.super.onText(webSocket, data, last);
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
