package com.bhf.aeroncache.models;

public class CacheDetails {
    private String cacheId;
    private long itemCount;

    public CacheDetails() {}

    public CacheDetails(String cacheId, long itemCount) {
        this.cacheId = cacheId;
        this.itemCount = itemCount;
    }

    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }

    public long getItemCount() { return itemCount; }
    public void setItemCount(long itemCount) { this.itemCount = itemCount; }
}
