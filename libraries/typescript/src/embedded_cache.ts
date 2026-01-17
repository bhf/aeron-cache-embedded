import { AeronCacheClient } from './index';

export class EmbeddedAeronCache {
    private localCache: Map<string, string> = new Map();

    constructor(private client: AeronCacheClient, private cacheId: string) {}

    getLocal(key: string): string | undefined {
        return this.localCache.get(key);
    }

    async put(key: string, value: string): Promise<any> {
        return this.client.putItem(this.cacheId, key, value);
    }

    async get(key: string): Promise<any> {
        return this.client.getItem(this.cacheId, key);
    }

    async delete(key: string): Promise<any> {
        return this.client.deleteItem(this.cacheId, key);
    }

    async clear(): Promise<any> {
        return this.client.deleteCache(this.cacheId);
    }

    subscribe(onMessage: (data: any) => void, onError?: (err: any) => void): WebSocket {
        const wrappedOnMessage = (data: any) => {
            this.updateLocalCache(data);
            onMessage(data);
        };
        return this.client.subscribe(this.cacheId, wrappedOnMessage, onError);
    }

    private updateLocalCache(event: any) {
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
