import { EmbeddedAeronCache } from './embedded_cache';
import { EmbeddedCounterCache } from './embedded_counter_cache';
import {
    CreateResponse,
    PutItemResponse,
    GetItemResponse,
    DeleteItemResponse,
    DeleteCacheResponse,
    CacheUpdateEvent,
    GetCacheResponse,
    ClearCacheResponse,
    BulkCacheOpsRequest,
    BulkCacheOpsResponse,
    CounterResponse,
    CounterUpdateEvent
} from './models';

export { EmbeddedAeronCache };
export { EmbeddedCounterCache };
export * from './models';

export class AeronCacheClient {
    private baseUrl: string;
    private wsUrl: string;

    constructor(baseUrl: string, wsUrl: string) {
        this.baseUrl = baseUrl;
        this.wsUrl = wsUrl;
    }

    // Note: TypeScript/JavaScript is inherently asynchronous for Network I/O.
    // Sync operations are not supported in standard environment without blocking the event loop.
    // Providing Async methods.

    async createCache(cacheId: string): Promise<CreateResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/cache`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ cacheId })
        });
        return this.handleResponse(response);
    }

    async putItem(cacheId: string, key: string, value: string): Promise<PutItemResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/cache/${cacheId}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key, value })
        });
        return this.handleResponse(response);
    }

    async putTimedItem(cacheId: string, key: string, value: string, ttl: number): Promise<PutItemResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/cache/timed/${cacheId}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key, value, ttl })
        });
        return this.handleResponse(response);
    }

    async getItem(cacheId: string, key: string): Promise<GetItemResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/cache/${cacheId}/${key}`);
        return this.handleResponse(response);
    }

    async deleteItem(cacheId: string, key: string): Promise<DeleteItemResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/cache/${cacheId}/${key}`, {
            method: 'DELETE'
        });
        return this.handleResponse(response);
    }

    
    async getCacheItems(cacheId: string): Promise<GetCacheResponse> {
        const url = `${this.baseUrl}/api/v1/cache/${cacheId}`;
        const response = await fetch(url);
        
        if (!response.ok && response.status !== 404 && response.status !== 400) {
            throw new Error(`Http Error: ${response.status}`);
        }
        
        return response.json();
    }

    async clearCache(cacheId: string): Promise<ClearCacheResponse> {
        const url = `${this.baseUrl}/api/v1/cache/${cacheId}`;
        const response = await fetch(url, { method: 'PATCH' });
        
        if (!response.ok && response.status !== 404 && response.status !== 400) {
            throw new Error(`Http Error: ${response.status}`);
        }
        
        return response.json();
    }

    async deleteCache(cacheId: string): Promise<DeleteCacheResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/cache/${cacheId}`, {
            method: 'DELETE'
        });
        return this.handleResponse(response);
    }

    async bulkOps(request: BulkCacheOpsRequest): Promise<BulkCacheOpsResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/cache/bulkops`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(request)
        });
        return this.handleResponse(response);
    }

    getCache(cacheId: string): EmbeddedAeronCache {
        return new EmbeddedAeronCache(this, cacheId);
    }

    // --- Counter Operations ---

    async createCounterCache(cacheId: string): Promise<CreateResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/counters/`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ cacheId })
        });
        return this.handleResponse(response);
    }

    async putCounter(cacheId: string, key: string, value: number): Promise<PutItemResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/counters/${cacheId}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key, value })
        });
        return this.handleResponse(response);
    }

    async putTimedCounter(cacheId: string, key: string, value: number, ttl: number): Promise<PutItemResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/counters/timed/${cacheId}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key, value, ttl })
        });
        return this.handleResponse(response);
    }

    async getCounter(cacheId: string, key: string): Promise<CounterResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/counters/${cacheId}/${key}`);
        return this.handleResponse(response);
    }

    async deleteCounter(cacheId: string, key: string): Promise<DeleteItemResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/counters/${cacheId}/${key}`, {
            method: 'DELETE'
        });
        return this.handleResponse(response);
    }

    async deleteCounterCache(cacheId: string): Promise<DeleteCacheResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/counters/${cacheId}`, {
            method: 'DELETE'
        });
        return this.handleResponse(response);
    }

    async incrementCounter(cacheId: string, key: string, amount: number): Promise<CounterResponse> {
        return this.counterAmountOp('increment', cacheId, key, amount);
    }

    async decrementCounter(cacheId: string, key: string, amount: number): Promise<CounterResponse> {
        return this.counterAmountOp('decrement', cacheId, key, amount);
    }

    private async counterAmountOp(op: string, cacheId: string, key: string, amount: number): Promise<CounterResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/counters/${op}/${cacheId}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key, amount })
        });
        return this.handleResponse(response);
    }

    async setCounter(cacheId: string, key: string, value: number): Promise<CounterResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/counters/set/${cacheId}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key, value })
        });
        return this.handleResponse(response);
    }

    getCounterCache(cacheId: string): EmbeddedCounterCache {
        return new EmbeddedCounterCache(this, cacheId);
    }

    subscribe(
        cacheIds: string,
        onMessage: (data: CacheUpdateEvent) => void,
        onError?: (err: any) => void,
        onStatusChange?: (status: 'Connected' | 'Disconnected') => void,
        hydrate: boolean = false
    ): { close: () => void } {
        const prefix = hydrate ?
            (cacheIds.includes(',') ? '/api/ws/v1/caches/hydrate' : '/api/ws/v1/cache/hydrate') :
            (cacheIds.includes(',') ? '/api/ws/v1/caches' : '/api/ws/v1/cache');
        return this.openSocket<CacheUpdateEvent>(`${prefix}/${cacheIds}`, onMessage, onError, onStatusChange);
    }

    subscribeCounter(
        cacheIds: string,
        onMessage: (data: CounterUpdateEvent) => void,
        onError?: (err: any) => void,
        onStatusChange?: (status: 'Connected' | 'Disconnected') => void,
        hydrate: boolean = false
    ): { close: () => void } {
        const prefix = hydrate ?
            (cacheIds.includes(',') ? '/api/ws/v1/counters/hydrate' : '/api/ws/v1/counter/hydrate') :
            (cacheIds.includes(',') ? '/api/ws/v1/counters' : '/api/ws/v1/counter');
        return this.openSocket<CounterUpdateEvent>(`${prefix}/${cacheIds}`, onMessage, onError, onStatusChange);
    }

    private openSocket<T>(
        path: string,
        onMessage: (data: T) => void,
        onError?: (err: any) => void,
        onStatusChange?: (status: 'Connected' | 'Disconnected') => void
    ): { close: () => void } {
        let ws: WebSocket | null = null;
        let isClosed = false;
        let reconnectTimeout: any = null;
        const wsUrl = `${this.wsUrl.replace(/\/$/, '')}${path}`;

        const connect = () => {
            if (isClosed) return;

            ws = new WebSocket(wsUrl);

            ws.onopen = () => {
                if (onStatusChange) onStatusChange('Connected');
            };

            ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data) as T;
                    onMessage(data);
                } catch (e) {
                    if (onError) onError(e);
                }
            };

            ws.onerror = (err) => {
                if (onError) onError(err);
            };

            ws.onclose = () => {
                if (onStatusChange) onStatusChange('Disconnected');
                if (!isClosed) {
                    reconnectTimeout = setTimeout(connect, 5000);
                }
            };
        };

        connect();

        return {
            close: () => {
                isClosed = true;
                if (reconnectTimeout) clearTimeout(reconnectTimeout);
                if (ws) ws.close();
            }
        };
    }

    private async handleResponse(response: Response): Promise<any> {
        const allowStatus = [200, 201, 400, 401, 404];
        if (!response.ok && !allowStatus.includes(response.status)) {
            throw new Error(`HTTP Error: ${response.status} ${response.statusText}`);
        }
        return response.json();
    }
}
