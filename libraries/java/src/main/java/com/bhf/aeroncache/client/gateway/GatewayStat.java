package com.bhf.aeroncache.client.gateway;

/**
 * A single cache's stats as streamed in a {@code GatewayStats} frame.
 */
public record GatewayStat(String cacheId, long addedCount, long removedCount, long clearedCount, long size) {
}
