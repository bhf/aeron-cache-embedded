package com.bhf.aeroncache.models;

public class CacheOperationRequest {
    private BulkOperationType operationType;
    private Long ttl;
    private String requestId;
    private String cacheId;
    private String key;
    private String value;

    public BulkOperationType getOperationType() { return operationType; }
    public void setOperationType(BulkOperationType operationType) { this.operationType = operationType; }

    public Long getTtl() { return ttl; }
    public void setTtl(Long ttl) { this.ttl = ttl; }

    public String getRequestId() { return requestId; }
    public void setRequestId(String requestId) { this.requestId = requestId; }

    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }

    public String getKey() { return key; }
    public void setKey(String key) { this.key = key; }

    public String getValue() { return value; }
    public void setValue(String value) { this.value = value; }

    public static Builder builder() { return new Builder(); }

    public static class Builder {
        private CacheOperationRequest request = new CacheOperationRequest();
        public Builder operationType(BulkOperationType type) { request.setOperationType(type); return this; }
        public Builder ttl(Long ttl) { request.setTtl(ttl); return this; }
        public Builder requestId(String id) { request.setRequestId(id); return this; }
        public Builder cacheId(String id) { request.setCacheId(id); return this; }
        public Builder key(String key) { request.setKey(key); return this; }
        public Builder value(String value) { request.setValue(value); return this; }
        public CacheOperationRequest build() { return request; }
    }
}
