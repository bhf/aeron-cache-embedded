package com.bhf.aeroncache.client.bidi;

/**
 * A handle to an active bidi WebSocket subscription. Closing it unsubscribes from streaming updates.
 */
public final class BidiSubscription implements AutoCloseable {

    private final AeronBidiClient client;
    private final String correlationId;
    private final String cacheId;
    private final boolean counters;
    private volatile boolean closed;

    BidiSubscription(AeronBidiClient client, String correlationId, String cacheId, boolean counters) {
        this.client = client;
        this.correlationId = correlationId;
        this.cacheId = cacheId;
        this.counters = counters;
    }

    public String getCacheId() {
        return cacheId;
    }

    public boolean isCounters() {
        return counters;
    }

    @Override
    public void close() {
        if (!closed) {
            closed = true;
            client.unsubscribe(correlationId, cacheId, counters);
        }
    }
}
