package com.aeron.cache.models;

public class DeleteCacheResponse {
    private String cacheId;
    private OperationStatus operationStatus;

    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }

    public OperationStatus getOperationStatus() { return operationStatus; }
    public void setOperationStatus(OperationStatus operationStatus) { this.operationStatus = operationStatus; }
}
