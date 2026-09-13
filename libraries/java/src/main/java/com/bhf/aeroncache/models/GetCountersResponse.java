package com.bhf.aeroncache.models;

import java.util.List;

public class GetCountersResponse {
    private String cacheId;
    private String operationStatus;
    private List<CounterItem> items;

    public GetCountersResponse() {}

    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }

    public String getOperationStatus() { return operationStatus; }
    public void setOperationStatus(String operationStatus) { this.operationStatus = operationStatus; }

    public List<CounterItem> getItems() { return items; }
    public void setItems(List<CounterItem> items) { this.items = items; }
}
