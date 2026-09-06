package com.aeron.cache.models;

import java.util.ArrayList;
import java.util.List;

public class BulkCacheOpsResponse {
    private String requestId;
    private List<CacheOperationResponse> operationResponses = new ArrayList<>();

    public String getRequestId() { return requestId; }
    public void setRequestId(String requestId) { this.requestId = requestId; }
    public List<CacheOperationResponse> getOperationResponses() { return operationResponses; }
    public void setOperationResponses(List<CacheOperationResponse> operationResponses) { this.operationResponses = operationResponses; }
}
