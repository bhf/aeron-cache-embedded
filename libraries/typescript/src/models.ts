export interface CreateRequest {
    cacheId: string;
}

export interface CreateResponse {
    cacheId: string;
    operationStatus: string;
}

export interface PutItemRequest {
    key: string;
    value: string;
}

export interface PutTimedItemRequest {
    key: string;
    value: string;
    ttl: number;
}

export interface PutItemResponse {
    cacheId: string;
    key: string;
    operationStatus: string;
}

export interface GetItemResponse {
    cacheId: string;
    key: string;
    value: string;
    operationStatus: string;
}

export interface DeleteItemResponse {
    cacheId: string;
    key: string;
    operationStatus: string;
}

export interface DeleteCacheResponse {
    cacheId: string;
    operationStatus: string;
}

export interface CacheUpdateEvent {
    cacheId: string;
    eventType: string;
    requestId: string;
    itemKey?: string;
    itemValue?: string;
}

// --- Counters ---

// Counter values are 64-bit integers on the server, represented here as `number`;
// callers dealing with values beyond Number.MAX_SAFE_INTEGER should be aware of
// the usual JavaScript precision limits.
export interface CounterResponse {
    cacheId: string;
    key: string;
    value: number;
    operationStatus: string;
}

export interface PutCounterRequest {
    key: string;
    value: number;
}

export interface PutTimedCounterRequest {
    key: string;
    value: number;
    ttl: number;
}

export interface IncrementCounterRequest {
    key: string;
    amount: number;
}

export interface CounterUpdateEvent {
    cacheId: string;
    eventType: string;
    requestId: string;
    itemKey?: string;
    itemValue?: number;
}

export interface CacheItem {
    key: string;
    value: string;
}

export interface GetCacheResponse {
    cacheId: string;
    operationStatus: string;
    items: CacheItem[];
}

export interface ClearCacheResponse {
    cacheId: string;
    operationStatus: string;
}

export type BulkOperationType =
    | 'NONE'
    | 'CREATE_CACHE'
    | 'ADD_ITEM'
    | 'REMOVE_ITEM'
    | 'CLEAR_CACHE'
    | 'GET_ITEM'
    | 'DELETE_CACHE'
    | 'CREATE_COUNTER_CACHE'
    | 'ADD_COUNTER'
    | 'REMOVE_COUNTER'
    | 'CLEAR_COUNTER_CACHE'
    | 'GET_COUNTER'
    | 'DELETE_COUNTER_CACHE'
    | 'INCREMENT_COUNTER'
    | 'DECREMENT_COUNTER'
    | 'SET_COUNTER';

export interface CacheOperationRequest {
    operationType: BulkOperationType;
    requestId: string;
    cacheId: string;
    key?: string;
    value?: string;
    ttl?: number;
    counterValue?: number;
}

export interface BulkCacheOpsRequest {
    requestId: string;
    operations: CacheOperationRequest[];
}

export interface CacheOperationResponse {
    requestId: string;
    status: string;
    cacheId: string;
    key?: string;
    value?: string;
}

export interface BulkCacheOpsResponse {
    requestId: string;
    operationResponses: CacheOperationResponse[];
}
