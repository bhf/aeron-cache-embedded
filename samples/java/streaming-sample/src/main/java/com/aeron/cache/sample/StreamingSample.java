package com.aeron.cache.sample;


import com.aeron.cache.client.AeronCacheClient;
import com.aeron.cache.client.EmbeddedAeronCache;

public class StreamingSample {
    public static void main(String[] args) {
        String baseUrl = "http://localhost:7070";

        System.out.println("Starting Streaming Sample against " + baseUrl);

        // The EmbeddedAeronCache automatically connects to the WebSocket for updates.
        // We can register a listener or just observe the cache changing essentially 
        // by checking it periodically or if the public API exposed an observer.
        // Based on previous implementation, the cache updates itself. 
        // We will simulate a long-running process observing updates.

        var wsUrl = "http://localhost:7071";
        AeronCacheClient client = new AeronCacheClient(baseUrl, wsUrl);
        try {
            client.createCache("streaming-test-cache");
            System.out.println("Created cache 'streaming-test-cache'");
        } catch (Exception e) {
            e.printStackTrace();
        }
        EmbeddedAeronCache cache = new EmbeddedAeronCache(client, "streaming-test-cache");

        System.out.println("Listening for updates.");

        String lastValue = "";
        while (true) {
            // Poll a known key to see if it changes
            String current = null;
            try {
                current = cache.get("shared-key");
            } catch (Exception e) {
            }
            if (current!=null && !current.equals(lastValue)) {
                System.out.println("Observed change in 'shared-key': " + current);
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
