import { AeronCacheClient } from '../index';
import { AeronBidiClient } from '../bidi';

const SUCCESS = 'SUCCESS';

/**
 * Transport-neutral facade over the client libraries, so a single workload can drive the HTTP or
 * bidi-WebSocket client. Both expose identical async op methods returning the same model types, so
 * each implementation is a thin wrapper.
 *
 * Methods return the operation's {@code operationStatus} string where the workload asserts on it, the
 * read-back value for reads, or {@code null} for an absent key.
 */
export interface SoakClient {
    /** Transport label, used in the report and cache-id namespacing. */
    name(): string;

    createKvCache(cacheId: string): Promise<void>;
    createCounterCache(cacheId: string): Promise<void>;

    /** @returns operationStatus. */
    put(cacheId: string, key: string, value: string): Promise<string>;
    /** @returns the stored value, or null if the key is absent. */
    get(cacheId: string, key: string): Promise<string | null>;
    /** @returns operationStatus. */
    remove(cacheId: string, key: string): Promise<string>;
    /** @returns operationStatus. */
    clear(cacheId: string): Promise<string>;
    /** @returns operationStatus. */
    deleteCache(cacheId: string): Promise<string>;
    /** List the full contents of a key-value cache (used for bidirectional reconciliation). */
    listKv(cacheId: string): Promise<Map<string, string>>;

    /** @returns operationStatus. */
    putCounter(cacheId: string, key: string, value: number): Promise<string>;
    /** @returns the resulting counter value. */
    incrementCounter(cacheId: string, key: string, amount: number): Promise<number>;
    /** @returns the resulting counter value. */
    decrementCounter(cacheId: string, key: string, amount: number): Promise<number>;
    /** @returns the resulting counter value. */
    setCounter(cacheId: string, key: string, value: number): Promise<number>;
    /** @returns the counter value, or null if the key is absent. */
    getCounter(cacheId: string, key: string): Promise<number | null>;

    close(): Promise<void>;
}

/** {@link SoakClient} over the HTTP/REST {@link AeronCacheClient}. */
export class HttpSoakClient implements SoakClient {
    private readonly client: AeronCacheClient;

    constructor(baseUrl: string, wsUrl: string) {
        this.client = new AeronCacheClient(baseUrl, wsUrl);
    }

    name(): string {
        return 'http';
    }

    async createKvCache(cacheId: string): Promise<void> {
        await this.client.createCache(cacheId);
    }

    async createCounterCache(cacheId: string): Promise<void> {
        await this.client.createCounterCache(cacheId);
    }

    async put(cacheId: string, key: string, value: string): Promise<string> {
        return (await this.client.putItem(cacheId, key, value)).operationStatus;
    }

    async get(cacheId: string, key: string): Promise<string | null> {
        const r = await this.client.getItem(cacheId, key);
        return r.operationStatus === SUCCESS ? r.value : null;
    }

    async remove(cacheId: string, key: string): Promise<string> {
        return (await this.client.deleteItem(cacheId, key)).operationStatus;
    }

    async clear(cacheId: string): Promise<string> {
        return (await this.client.clearCache(cacheId)).operationStatus;
    }

    async deleteCache(cacheId: string): Promise<string> {
        return (await this.client.deleteCache(cacheId)).operationStatus;
    }

    async listKv(cacheId: string): Promise<Map<string, string>> {
        const out = new Map<string, string>();
        for (const item of (await this.client.getCacheItems(cacheId)).items) {
            out.set(item.key, item.value);
        }
        return out;
    }

    async putCounter(cacheId: string, key: string, value: number): Promise<string> {
        return (await this.client.putCounter(cacheId, key, value)).operationStatus;
    }

    async incrementCounter(cacheId: string, key: string, amount: number): Promise<number> {
        return (await this.client.incrementCounter(cacheId, key, amount)).value;
    }

    async decrementCounter(cacheId: string, key: string, amount: number): Promise<number> {
        return (await this.client.decrementCounter(cacheId, key, amount)).value;
    }

    async setCounter(cacheId: string, key: string, value: number): Promise<number> {
        return (await this.client.setCounter(cacheId, key, value)).value;
    }

    async getCounter(cacheId: string, key: string): Promise<number | null> {
        const r = await this.client.getCounter(cacheId, key);
        return r.operationStatus === SUCCESS ? r.value : null;
    }

    async close(): Promise<void> {
        // AeronCacheClient holds no long-lived connection to close.
    }
}

/** {@link SoakClient} over the bidirectional-WebSocket {@link AeronBidiClient}. */
export class BidiSoakClient implements SoakClient {
    private readonly client: AeronBidiClient;

    constructor(wsUrl: string) {
        this.client = new AeronBidiClient(wsUrl);
    }

    async connect(): Promise<void> {
        await this.client.connect();
    }

    name(): string {
        return 'bidi';
    }

    async createKvCache(cacheId: string): Promise<void> {
        await this.client.createCache(cacheId);
    }

    async createCounterCache(cacheId: string): Promise<void> {
        await this.client.createCounterCache(cacheId);
    }

    async put(cacheId: string, key: string, value: string): Promise<string> {
        return (await this.client.putItem(cacheId, key, value)).operationStatus;
    }

    async get(cacheId: string, key: string): Promise<string | null> {
        const r = await this.client.getItem(cacheId, key);
        return r.operationStatus === SUCCESS ? r.value : null;
    }

    async remove(cacheId: string, key: string): Promise<string> {
        return (await this.client.deleteItem(cacheId, key)).operationStatus;
    }

    async clear(cacheId: string): Promise<string> {
        return (await this.client.clearCache(cacheId)).operationStatus;
    }

    async deleteCache(cacheId: string): Promise<string> {
        return (await this.client.deleteCache(cacheId)).operationStatus;
    }

    async listKv(cacheId: string): Promise<Map<string, string>> {
        const out = new Map<string, string>();
        for (const item of (await this.client.getCacheItems(cacheId)).items) {
            out.set(item.key, item.value);
        }
        return out;
    }

    async putCounter(cacheId: string, key: string, value: number): Promise<string> {
        return (await this.client.putCounter(cacheId, key, value)).operationStatus;
    }

    async incrementCounter(cacheId: string, key: string, amount: number): Promise<number> {
        return (await this.client.incrementCounter(cacheId, key, amount)).value;
    }

    async decrementCounter(cacheId: string, key: string, amount: number): Promise<number> {
        return (await this.client.decrementCounter(cacheId, key, amount)).value;
    }

    async setCounter(cacheId: string, key: string, value: number): Promise<number> {
        return (await this.client.setCounter(cacheId, key, value)).value;
    }

    async getCounter(cacheId: string, key: string): Promise<number | null> {
        const r = await this.client.getCounter(cacheId, key);
        return r.operationStatus === SUCCESS ? r.value : null;
    }

    async close(): Promise<void> {
        await this.client.close();
    }
}
