import { AeronCacheClient } from './index';
import {
    PutItemResponse,
    GetItemResponse,
    DeleteItemResponse,
    DeleteCacheResponse,
    PatchItemResponse,
    CacheUpdateEvent
} from './models';

export type JsonObject = Record<string, any>;

/**
 * A cache view whose values are structured JSON objects rather than opaque strings, mirroring remote
 * updates into a local map.
 *
 * The key difference from {@link EmbeddedAeronCache} is how `PATCH_ITEM` events are applied: instead of
 * overwriting the entry with the delta, the delta is deep-merged into the stored object using RFC 7386
 * (JSON Merge Patch) semantics — nested objects merge recursively, scalars and arrays are replaced, and a
 * `null` field in the delta deletes that field. This matches the server-side `patchItem` deep-merge, so a
 * patch-mode subscription (which streams only the changed fields) reconstructs the full object locally
 * without losing untouched fields.
 */
export class EmbeddedObjectCache {
    private localCache: Map<string, JsonObject> = new Map();

    constructor(private client: AeronCacheClient, private cacheId: string) {}

    /** The locally mirrored object for `key`, or `undefined` if absent. */
    getLocal<T extends JsonObject = JsonObject>(key: string): T | undefined {
        return this.localCache.get(key) as T | undefined;
    }

    async put(key: string, value: JsonObject): Promise<PutItemResponse> {
        return this.client.putItem(this.cacheId, key, JSON.stringify(value));
    }

    async putTimed(key: string, value: JsonObject, ttl: number): Promise<PutItemResponse> {
        return this.client.putTimedItem(this.cacheId, key, JSON.stringify(value), ttl);
    }

    /**
     * Deep-merge a JSON fragment into the stored object (RFC 7386). A `null`-valued field deletes that field.
     */
    async patch(key: string, fragment: JsonObject): Promise<PatchItemResponse> {
        return this.client.patchItem(this.cacheId, key, JSON.stringify(fragment));
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
        hydrate: boolean = false,
        keys?: string,
        mode?: 'full' | 'patch'
    ): { close: () => void } {
        const wrappedOnMessage = (data: CacheUpdateEvent) => {
            this.updateLocalCache(data);
            onMessage(data);
        };
        return this.client.subscribe(this.cacheId, wrappedOnMessage, onError, onStatusChange, hydrate, keys, mode);
    }

    private updateLocalCache(event: CacheUpdateEvent) {
        if (!event || !event.eventType) return;

        switch (event.eventType) {
            case 'ADD_ITEM':
                if (event.itemKey && event.itemValue) {
                    this.localCache.set(event.itemKey, EmbeddedObjectCache.parseObject(event.itemValue));
                }
                break;
            case 'PATCH_ITEM':
                if (event.itemKey && event.itemValue) {
                    const delta = EmbeddedObjectCache.parseObject(event.itemValue);
                    const existing = this.localCache.get(event.itemKey) ?? {};
                    this.localCache.set(event.itemKey, EmbeddedObjectCache.deepMerge(existing, delta));
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

    private static parseObject(json: string): JsonObject {
        const parsed = JSON.parse(json);
        if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
            throw new Error(`EmbeddedObjectCache values must be JSON objects, got: ${json}`);
        }
        return parsed;
    }

    /**
     * RFC 7386 (JSON Merge Patch) deep-merge of `patch` into `target`, returning a new object. Nested
     * objects merge recursively; scalars and arrays replace; a `null` field deletes the field.
     */
    static deepMerge(target: JsonObject, patch: JsonObject): JsonObject {
        const result: JsonObject = { ...target };
        for (const [name, value] of Object.entries(patch)) {
            if (value === null) {
                delete result[name];
            } else if (typeof value === 'object' && !Array.isArray(value)) {
                const existing = result[name];
                const base = (existing && typeof existing === 'object' && !Array.isArray(existing)) ? existing : {};
                result[name] = EmbeddedObjectCache.deepMerge(base, value);
            } else {
                result[name] = value;
            }
        }
        return result;
    }
}
