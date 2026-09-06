package com.bhf.aeroncache.models;

/**
 * Response for counter operations that return a value (get, increment, decrement, set).
 */
public class CounterResponse {
    private String cacheId;
    private String key;
    private long value;
    private String operationStatus;

    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }
    public String getKey() { return key; }
    public void setKey(String key) { this.key = key; }
    public long getValue() { return value; }
    public void setValue(long value) { this.value = value; }

    public String getOperationStatus() { return operationStatus; }
    public void setOperationStatus(String operationStatus) { this.operationStatus = operationStatus; }
}
