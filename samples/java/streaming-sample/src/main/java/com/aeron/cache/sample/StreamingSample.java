package com.aeron.cache.sample;

import com.aeron.cache.embedded.EmbeddedAeronCache;

public class StreamingSample {
    public static void main(String[] args) {
        String baseUrl = "http://localhost:8080";
        if (args.length > 0) {
            baseUrl = args[0];
        }
        
        System.out.println("Starting Streaming Sample against " + baseUrl);
        
        // The EmbeddedAeronCache automatically connects to the WebSocket for updates.
        // We can register a listener or just observe the cache changing essentially 
        // by checking it periodically or if the public API exposed an observer.
        // Based on previous implementation, the cache updates itself. 
        // We will simulate a long-running process observing updates.

        try (EmbeddedAeronCache cache = new EmbeddedAeronCache(baseUrl)) {
            System.out.println("Listening for updates. Press Ctrl+C to exit.");
            
            String lastValue = "";
            while (true) {
                // Poll a known key to see if it changes
                String current = cache.get("shared-key").orElse("null");
                if (!current.equals(lastValue)) {
                    System.out.println("Observed change in 'shared-key': " + current);
                    lastValue = current;
                }
                
                Thread.sleep(1000);
            }
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
