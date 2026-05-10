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
