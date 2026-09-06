package com.aeron.cache.models;

import com.fasterxml.jackson.annotation.JsonInclude;

/**
 * A single operation within a {@link BulkCacheOpsRequest}. Unset fields are
 * omitted from the serialized payload.
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public class CacheOperationRequest {
    private BulkOperationType operationType;
    private Long ttl;
    private Long counterValue;
    private String requestId;
    private String cacheId;
    private String key;
    private String value;

    public CacheOperationRequest() {}

    public CacheOperationRequest(BulkOperationType operationType, String cacheId, String key, String value) {
        this.operationType = operationType;
        this.cacheId = cacheId;
        this.key = key;
        this.value = value;
    }

    public BulkOperationType getOperationType() { return operationType; }
    public void setOperationType(BulkOperationType operationType) { this.operationType = operationType; }
    public Long getTtl() { return ttl; }
    public void setTtl(Long ttl) { this.ttl = ttl; }
    public Long getCounterValue() { return counterValue; }
    public void setCounterValue(Long counterValue) { this.counterValue = counterValue; }
    public String getRequestId() { return requestId; }
    public void setRequestId(String requestId) { this.requestId = requestId; }
    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }
    public String getKey() { return key; }
    public void setKey(String key) { this.key = key; }
    public String getValue() { return value; }
    public void setValue(String value) { this.value = value; }
}
