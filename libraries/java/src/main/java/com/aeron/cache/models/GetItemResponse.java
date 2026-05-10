package com.aeron.cache.models;

public class GetItemResponse {
    private String cacheId;
    private String key;
    private String value;
    private String operationStatus;

    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }
    public String getKey() { return key; }
    public void setKey(String key) { this.key = key; }
    public String getValue() { return value; }
    public void setValue(String value) { this.value = value; }

    public String getOperationStatus() { return operationStatus; }
    public void setOperationStatus(String operationStatus) { this.operationStatus = operationStatus; }
}
