package com.aeron.cache.models;

/**
 * A counter cache update event published over the WebSocket connection.
 * Mirrors {@link CacheUpdateEvent} but carries a numeric (int64) item value.
 */
public class CounterUpdateEvent {
    private String cacheId;
    private String eventType;
    private String itemKey;
    private Long itemValue;
    private String requestId;

    public String getCacheId() { return cacheId; }
    public void setCacheId(String cacheId) { this.cacheId = cacheId; }
    public String getEventType() { return eventType; }
    public void setEventType(String eventType) { this.eventType = eventType; }
    public String getItemKey() { return itemKey; }
    public void setItemKey(String itemKey) { this.itemKey = itemKey; }
    public Long getItemValue() { return itemValue; }
    public void setItemValue(Long itemValue) { this.itemValue = itemValue; }
    public String getRequestId() { return requestId; }
    public void setRequestId(String requestId) { this.requestId = requestId; }
}
