package com.bhf.aeroncache.client.gateway;

/**
 * A handle to an active gateway subscription. Closing it unsubscribes from streaming updates.
 */
public final class GatewaySubscription implements AutoCloseable {

    private final String cacheId;
    private final boolean counters;
    private final Runnable onClose;
    private volatile boolean closed;

    GatewaySubscription(String cacheId, boolean counters, Runnable onClose) {
        this.cacheId = cacheId;
        this.counters = counters;
        this.onClose = onClose;
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
            onClose.run();
        }
    }
}
