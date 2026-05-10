export type OperationStatus = 'SUCCESS' | 'ERROR' | 'UNKNOWN_CACHE' | 'UNKNOWN_KEY' | 'CACHE_EXISTS';

export interface CreateResponse {
    cacheId: string;
    operationStatus: string;
}

export interface PutItemRequest {
    key: string;
    value: string;
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
