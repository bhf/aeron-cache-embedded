package com.bhf.aeroncache.models;

public class CounterItem {
    private String key;
    private long value;

    public CounterItem() {}

    public CounterItem(String key, long value) {
        this.key = key;
        this.value = value;
    }

    public String getKey() { return key; }
    public void setKey(String key) { this.key = key; }

    public long getValue() { return value; }
    public void setValue(long value) { this.value = value; }
}
