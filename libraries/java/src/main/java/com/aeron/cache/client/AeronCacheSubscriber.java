package com.aeron.cache.client;

import com.aeron.cache.models.CacheUpdateEvent;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.net.http.WebSocket;
import java.util.concurrent.CompletionStage;
import java.util.function.Consumer;

/**
 * An abstraction for subscribing to cache updates.
 * It handles the WebSocket request(1) calls and provides lifecycle hooks.
 */
public abstract class AeronCacheSubscriber implements WebSocket.Listener {
    private final ObjectMapper objectMapper = new ObjectMapper();
    private Consumer<CacheUpdateEvent> internalUpdater;

    /**
     * Internal method used by EmbeddedAeronCache to inject the local cache update logic.
     */
    void setInternalUpdater(Consumer<CacheUpdateEvent> internalUpdater) {
        this.internalUpdater = internalUpdater;
    }

    /**
     * Called before the local cache is updated with the event.
     * @param event The update event.
     */
    public void onBeforeUpdate(CacheUpdateEvent event) {}

    /**
     * Called after the local cache has been updated with the event.
     * @param event The update event.
     */
    public void onAfterUpdate(CacheUpdateEvent event) {}

    @Override
    public void onOpen(WebSocket webSocket) {
        webSocket.request(1);
    }

    @Override
    public CompletionStage<?> onText(WebSocket webSocket, CharSequence data, boolean last) {
        String payload = data.toString();
        try {
            CacheUpdateEvent event = objectMapper.readValue(payload, CacheUpdateEvent.class);
            
            onBeforeUpdate(event);
            
            if (internalUpdater != null) {
                internalUpdater.accept(event);
            }
            
            onAfterUpdate(event);
        } catch (Exception e) {
            onError(webSocket, e);
        }
        
        webSocket.request(1);
        return null;
    }

    @Override
    public void onError(WebSocket webSocket, Throwable error) {
        error.printStackTrace();
    }
}
