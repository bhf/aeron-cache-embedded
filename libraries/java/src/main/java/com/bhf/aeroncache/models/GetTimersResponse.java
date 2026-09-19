package com.bhf.aeroncache.models;

import java.util.List;

/**
 * The response from getting all pending TTL removal timers across caches and counter caches, as
 * returned by the HTTP transport's {@code GET /api/v1/timers} endpoint.
 */
public class GetTimersResponse {
    private String operationStatus;
    private List<TimerInfo> timers;

    public GetTimersResponse() {}

    public String getOperationStatus() { return operationStatus; }
    public void setOperationStatus(String operationStatus) { this.operationStatus = operationStatus; }

    public List<TimerInfo> getTimers() { return timers; }
    public void setTimers(List<TimerInfo> timers) { this.timers = timers; }
}
