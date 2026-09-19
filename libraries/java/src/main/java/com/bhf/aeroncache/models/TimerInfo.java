package com.bhf.aeroncache.models;

/**
 * A single pending TTL removal timer, as returned by the {@code getTimers} operation across all
 * transports (HTTP {@code GET /api/v1/timers}, the bidirectional WebSocket {@code timers} frame, and
 * the Aeron gateway {@code GatewayTimers} message).
 * <p>
 * Each timer is tagged with its {@code timerType} ({@code CACHE} or {@code COUNTER}) so cache timers
 * can be told from counter timers.
 */
public class TimerInfo {
    private String timerType;
    private String cacheId;
    private String key;
    private long deadline;

    public TimerInfo() {}

    public TimerInfo(String timerType, String cacheId, String key, long deadline) {
        this.timerType = timerType;
        this.cacheId = cacheId;
        this.key = key;
        this.deadline = deadline;
    }

    public String getTimerType() { return timerType; }
    public void setTimerType(String timerType) { this.timerType = timerType; }

    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }

    public String getKey() { return key; }
    public void setKey(String key) { this.key = key; }

    /** @return the epoch time (millis) at which the removal is scheduled to fire. */
    public long getDeadline() { return deadline; }
    public void setDeadline(long deadline) { this.deadline = deadline; }
}
