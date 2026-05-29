package com.bhf.aeroncache.models;

import java.util.List;

public class BulkCacheOpsRequest {
    private String requestId;
    private List<CacheOperationRequest> operations;

    public String getRequestId() { return requestId; }
    public void setRequestId(String requestId) { this.requestId = requestId; }

    public List<CacheOperationRequest> getOperations() { return operations; }
    public void setOperations(List<CacheOperationRequest> operations) { this.operations = operations; }

    public static Builder builder() { return new Builder(); }

    public static class Builder {
        private BulkCacheOpsRequest request = new BulkCacheOpsRequest();
        private java.util.List<CacheOperationRequest> operations = new java.util.ArrayList<>();

        public Builder requestId(String id) { request.setRequestId(id); return this; }
        public Builder operations(java.util.List<CacheOperationRequest> ops) { this.operations = ops; return this; }
        public Builder addOperation(CacheOperationRequest op) { this.operations.add(op); return this; }
        
        public BulkCacheOpsRequest build() { 
            request.setOperations(operations);
            return request; 
        }
    }
}
