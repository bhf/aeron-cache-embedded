package com.aeron.cache.models;

public class DeleteItemResponse {
    private String cacheId;
    private String key;
    private OperationStatus operationStatus;

    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }
    public String getKey() { return key; }
    public void setKey(String key) { this.key = key; }

    public OperationStatus getOperationStatus() { return operationStatus; }
    public void setOperationStatus(OperationStatus operationStatus) { this.operationStatus = operationStatus; }
}
