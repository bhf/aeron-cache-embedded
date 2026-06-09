import { AeronCacheClient } from './index';
import { 
    PutItemResponse, 
    GetItemResponse, 
    DeleteItemResponse, 
    DeleteCacheResponse,
    CacheUpdateEvent
} from './models';

export class EmbeddedAeronCache {
    private localCache: Map<string, string> = new Map();

    constructor(private client: AeronCacheClient, private cacheId: string) {}

    getLocal(key: string): string | undefined {
        return this.localCache.get(key);
    }

    async put(key: string, value: string): Promise<PutItemResponse> {
        return this.client.putItem(this.cacheId, key, value);
    }

    async putTimed(key: string, value: string, ttl: number): Promise<PutItemResponse> {
        return this.client.putTimedItem(this.cacheId, key, value, ttl);
    }

    async get(key: string): Promise<GetItemResponse> {
        return this.client.getItem(this.cacheId, key);
    }

    async delete(key: string): Promise<DeleteItemResponse> {
        return this.client.deleteItem(this.cacheId, key);
    }

    async clear(): Promise<DeleteCacheResponse> {
        return this.client.deleteCache(this.cacheId);
    }

    subscribe(
        onMessage: (data: CacheUpdateEvent) => void, 
        onError?: (err: any) => void,
        onStatusChange?: (status: 'Connected' | 'Disconnected') => void,
        hydrate: boolean = false
    ): { close: () => void } {
        const wrappedOnMessage = (data: CacheUpdateEvent) => {
            this.updateLocalCache(data);
            onMessage(data);
        };
        return this.client.subscribe(this.cacheId, wrappedOnMessage, onError, onStatusChange, hydrate);
    }

    private updateLocalCache(event: CacheUpdateEvent) {
        if (!event || !event.eventType) return;

        switch (event.eventType) {
            case 'ADD_ITEM':
                if (event.itemKey && event.itemValue) {
                    this.localCache.set(event.itemKey, event.itemValue);
                }
                break;
            case 'REMOVE_ITEM':
                if (event.itemKey) {
                    this.localCache.delete(event.itemKey);
                }
                break;
            case 'CLEAR_CACHE':
            case 'DELETE_CACHE':
                this.localCache.clear();
                break;
        }
    }
}
