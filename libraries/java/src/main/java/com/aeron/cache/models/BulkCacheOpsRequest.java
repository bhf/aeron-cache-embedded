package com.aeron.cache.models;

import com.fasterxml.jackson.annotation.JsonInclude;

import java.util.ArrayList;
import java.util.List;

@JsonInclude(JsonInclude.Include.NON_NULL)
public class BulkCacheOpsRequest {
    private String requestId;
    private List<CacheOperationRequest> operations = new ArrayList<>();

    public BulkCacheOpsRequest() {}

    public BulkCacheOpsRequest(String requestId, List<CacheOperationRequest> operations) {
        this.requestId = requestId;
        this.operations = operations;
    }

    public String getRequestId() { return requestId; }
    public void setRequestId(String requestId) { this.requestId = requestId; }
    public List<CacheOperationRequest> getOperations() { return operations; }
    public void setOperations(List<CacheOperationRequest> operations) { this.operations = operations; }
}
