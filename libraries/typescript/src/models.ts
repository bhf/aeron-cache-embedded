export type OperationStatus =
    | 'NONE'
    | 'SUCCESS'
    | 'ERROR'
    | 'UNKNOWN_CACHE'
    | 'UNKNOWN_KEY'
    | 'CACHE_EXISTS'
    | 'DUPLICATE_SUBSCRIPTION'
    | 'UNKNOWN_SUBSCRIPTION'
    | 'NULL_VAL';

export interface CreateResponse {
    cacheId: string;
    operationStatus?: OperationStatus;
}

export interface PutItemRequest {
    cacheId: string;
    key: string;
    value: string;
}

export interface PutItemResponse {
    cacheId: string;
    key: string;
    operationStatus?: OperationStatus;
}

export interface GetItemResponse {
    cacheId: string;
    key: string;
    value: string;
    operationStatus?: OperationStatus;
}

export interface DeleteItemResponse {
    cacheId: string;
    key: string;
    operationStatus?: OperationStatus;
}

export interface DeleteCacheResponse {
    cacheId: string;
    operationStatus?: OperationStatus;
}

export interface CacheUpdateEvent {
    cacheId: string;
    key: string;
    value: string;
    timestamp: number;
}

export interface CacheUpdateEvent {
    cacheId: string;
    eventType: 'ADD_ITEM' | 'DELETE_CACHE' | 'REMOVE_ITEM' | 'CLEAR_CACHE';
    itemKey?: string;
    itemValue?: string;
    requestId: string;
}

// --- Counters ---

// Counter values are 64-bit integers on the server. They are represented as
// `number` here; callers dealing with values beyond Number.MAX_SAFE_INTEGER
// should be aware of the usual JavaScript precision limits.
export interface CounterResponse {
    cacheId: string;
    key: string;
    value: number;
    operationStatus?: OperationStatus;
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

export interface DecrementCounterRequest {
    key: string;
    amount: number;
}

export interface SetCounterRequest {
    key: string;
    value: number;
}

export interface CounterUpdateEvent {
    cacheId: string;
    eventType: 'ADD_ITEM' | 'DELETE_CACHE' | 'REMOVE_ITEM' | 'CLEAR_CACHE';
    itemKey?: string;
    itemValue?: number;
    requestId: string;
}

// --- Bulk operations ---

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
    cacheId?: string;
    key?: string;
    value?: string;
    ttl?: number;
    counterValue?: number;
    requestId?: string;
}

export interface BulkCacheOpsRequest {
    requestId?: string;
    operations: CacheOperationRequest[];
}

export interface CacheOperationResponse {
    requestId?: string;
    status?: OperationStatus;
    cacheId?: string;
    key?: string;
    value?: string;
}

export interface BulkCacheOpsResponse {
    requestId?: string;
    operationResponses: CacheOperationResponse[];
}
