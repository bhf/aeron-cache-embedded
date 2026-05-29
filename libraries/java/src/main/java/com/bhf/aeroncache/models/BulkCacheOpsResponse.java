package com.bhf.aeroncache.models;

import java.util.List;

public class BulkCacheOpsResponse {
    private String requestId;
    private List<CacheOperationResponse> operationResponses;

    public String getRequestId() { return requestId; }
    public void setRequestId(String requestId) { this.requestId = requestId; }

    public List<CacheOperationResponse> getOperationResponses() { return operationResponses; }
    public void setOperationResponses(List<CacheOperationResponse> operationResponses) { this.operationResponses = operationResponses; }
}
