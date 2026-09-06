package com.aeron.cache.models;

public enum OperationStatus {
    NONE,
    SUCCESS,
    ERROR,
    UNKNOWN_CACHE,
    UNKNOWN_KEY,
    CACHE_EXISTS,
    DUPLICATE_SUBSCRIPTION,
    UNKNOWN_SUBSCRIPTION,
    NULL_VAL
}
