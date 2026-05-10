package com.aeron.cache.models;

import java.util.List;

public class GetCacheResponse {
    private String cacheId;
    private String operationStatus;
    private List<CacheItem> items;

    public GetCacheResponse() {}

    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }

    public String getOperationStatus() { return operationStatus; }
    public void setOperationStatus(String operationStatus) { this.operationStatus = operationStatus; }

    public List<CacheItem> getItems() { return items; }
    public void setItems(List<CacheItem> items) { this.items = items; }
}
