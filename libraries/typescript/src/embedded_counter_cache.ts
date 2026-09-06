import { AeronCacheClient } from './index';
import {
    PutItemResponse,
    DeleteItemResponse,
    DeleteCacheResponse,
    CounterResponse,
    CounterUpdateEvent
} from './models';

export class EmbeddedCounterCache {
    private localCache: Map<string, number> = new Map();

    constructor(private client: AeronCacheClient, private cacheId: string) {}

    getLocal(key: string): number | undefined {
        return this.localCache.get(key);
    }

    async put(key: string, value: number): Promise<PutItemResponse> {
        return this.client.putCounter(this.cacheId, key, value);
    }

    async putTimed(key: string, value: number, ttl: number): Promise<PutItemResponse> {
        return this.client.putTimedCounter(this.cacheId, key, value, ttl);
    }

    async get(key: string): Promise<CounterResponse> {
        return this.client.getCounter(this.cacheId, key);
    }

    async increment(key: string, amount: number): Promise<CounterResponse> {
        return this.client.incrementCounter(this.cacheId, key, amount);
    }

    async decrement(key: string, amount: number): Promise<CounterResponse> {
        return this.client.decrementCounter(this.cacheId, key, amount);
    }

    async set(key: string, value: number): Promise<CounterResponse> {
        return this.client.setCounter(this.cacheId, key, value);
    }

    async delete(key: string): Promise<DeleteItemResponse> {
        return this.client.deleteCounter(this.cacheId, key);
    }

    async clear(): Promise<DeleteCacheResponse> {
        return this.client.deleteCounterCache(this.cacheId);
    }

    subscribe(
        onMessage: (data: CounterUpdateEvent) => void,
        onError?: (err: any) => void,
        onStatusChange?: (status: 'Connected' | 'Disconnected') => void,
        hydrate: boolean = false
    ): { close: () => void } {
        const wrappedOnMessage = (data: CounterUpdateEvent) => {
            this.updateLocalCache(data);
            onMessage(data);
        };
        return this.client.subscribeCounter(this.cacheId, wrappedOnMessage, onError, onStatusChange, hydrate);
    }

    private updateLocalCache(event: CounterUpdateEvent) {
        if (!event || !event.eventType) return;

        switch (event.eventType) {
            case 'ADD_ITEM':
                // `!= null` so that a counter value of 0 is stored, not skipped.
                if (event.itemKey != null && event.itemValue != null) {
                    this.localCache.set(event.itemKey, event.itemValue);
                }
                break;
            case 'REMOVE_ITEM':
                if (event.itemKey != null) {
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
