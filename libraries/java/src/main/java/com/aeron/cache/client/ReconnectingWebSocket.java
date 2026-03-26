package com.aeron.cache.client;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.WebSocket;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

public class ReconnectingWebSocket {
    private final HttpClient httpClient;
    private final URI uri;
    private final WebSocket.Listener listener;
    private final ScheduledExecutorService scheduler = Executors.newSingleThreadScheduledExecutor();
    private final AtomicBoolean isClosed = new AtomicBoolean(false);
    private WebSocket webSocket;

    public ReconnectingWebSocket(HttpClient httpClient, URI uri, WebSocket.Listener listener) {
        this.httpClient = httpClient;
        this.uri = uri;
        this.listener = listener;
        connect();
    }

    private void connect() {
        if (isClosed.get()) return;

        httpClient.newWebSocketBuilder()
                .buildAsync(uri, new InternalListener())
                .whenComplete((ws, ex) -> {
                    if (ex != null) {
                        scheduleReconnect();
                    } else {
                        this.webSocket = ws;
                    }
                });
    }

    private void scheduleReconnect() {
        if (isClosed.get()) return;
        scheduler.schedule(this::connect, 5, TimeUnit.SECONDS);
    }

    public void close() {
        isClosed.set(true);
        scheduler.shutdown();
        if (webSocket != null) {
            webSocket.sendClose(WebSocket.NORMAL_CLOSURE, "Closing");
        }
    }

    private class InternalListener implements WebSocket.Listener {
        @Override
        public void onOpen(WebSocket webSocket) {
            listener.onOpen(webSocket);
        }

        @Override
        public java.util.concurrent.CompletionStage<?> onText(WebSocket webSocket, CharSequence data, boolean last) {
            return listener.onText(webSocket, data, last);
        }

        @Override
        public java.util.concurrent.CompletionStage<?> onBinary(WebSocket webSocket, java.nio.ByteBuffer data, boolean last) {
            return listener.onBinary(webSocket, data, last);
        }

        @Override
        public java.util.concurrent.CompletionStage<?> onPing(WebSocket webSocket, java.nio.ByteBuffer message) {
            return listener.onPing(webSocket, message);
        }

        @Override
        public java.util.concurrent.CompletionStage<?> onPong(WebSocket webSocket, java.nio.ByteBuffer message) {
            return listener.onPong(webSocket, message);
        }

        @Override
        public void onError(WebSocket webSocket, Throwable error) {
            listener.onError(webSocket, error);
            scheduleReconnect();
        }

        @Override
        public java.util.concurrent.CompletionStage<?> onClose(WebSocket webSocket, int statusCode, String reason) {
            listener.onClose(webSocket, statusCode, reason);
            scheduleReconnect();
            return null;
        }
    }
}
