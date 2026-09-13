package com.bhf.aeroncache.models;

/**
 * Per-cache statistics entry, as streamed by the bidirectional WebSocket transport's
 * {@code GET_CACHE_STATS} / {@code GET_COUNTER_STATS} commands.
 * <p>
 * This is the per-cache breakdown carried in the bidi {@code stats} frames, as opposed to the HTTP
 * transport's aggregate {@link CacheStatsResponse}.
 */
public class StatEntry {
    private String cacheId;
    private long addedCount;
    private long removedCount;
    private long clearedCount;
    private long size;

    public StatEntry() {}

    public StatEntry(String cacheId, long addedCount, long removedCount, long clearedCount, long size) {
        this.cacheId = cacheId;
        this.addedCount = addedCount;
        this.removedCount = removedCount;
        this.clearedCount = clearedCount;
        this.size = size;
    }

    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }

    public long getAddedCount() { return addedCount; }
    public void setAddedCount(long addedCount) { this.addedCount = addedCount; }

    public long getRemovedCount() { return removedCount; }
    public void setRemovedCount(long removedCount) { this.removedCount = removedCount; }

    public long getClearedCount() { return clearedCount; }
    public void setClearedCount(long clearedCount) { this.clearedCount = clearedCount; }

    public long getSize() { return size; }
    public void setSize(long size) { this.size = size; }
}
