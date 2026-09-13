package com.bhf.aeroncache.models;

public class CacheStatsResponse {
    private int totalOpsCount;
    private int totalCachesCount;
    private int totalItemsCount;
    private int errorCount;

    public CacheStatsResponse() {}

    public int getTotalOpsCount() { return totalOpsCount; }
    public void setTotalOpsCount(int totalOpsCount) { this.totalOpsCount = totalOpsCount; }

    public int getTotalCachesCount() { return totalCachesCount; }
    public void setTotalCachesCount(int totalCachesCount) { this.totalCachesCount = totalCachesCount; }

    public int getTotalItemsCount() { return totalItemsCount; }
    public void setTotalItemsCount(int totalItemsCount) { this.totalItemsCount = totalItemsCount; }

    public int getErrorCount() { return errorCount; }
    public void setErrorCount(int errorCount) { this.errorCount = errorCount; }
}
