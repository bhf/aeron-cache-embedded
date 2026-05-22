package com.bhf.aeroncache.models;

public class DeleteCacheResponse {
    private String cacheId;
    private String operationStatus;

    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }

    public String getOperationStatus() { return operationStatus; }
    public void setOperationStatus(String operationStatus) { this.operationStatus = operationStatus; }
}
