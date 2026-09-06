import { EmbeddedAeronCache } from './embedded_cache';
import { EmbeddedCounterCache } from './embedded_counter_cache';
import {
    CreateResponse,
    PutItemResponse,
    GetItemResponse,
    DeleteItemResponse,
    DeleteCacheResponse,
    CacheUpdateEvent,
    CounterResponse,
    CounterUpdateEvent,
    BulkCacheOpsRequest,
    BulkCacheOpsResponse
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
            body: JSON.stringify({ cacheId, key, value })
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
        const response = await fetch(`${this.baseUrl}/api/v1/counters/increment/${cacheId}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key, amount })
        });
        return this.handleResponse(response);
    }

    async decrementCounter(cacheId: string, key: string, amount: number): Promise<CounterResponse> {
        const response = await fetch(`${this.baseUrl}/api/v1/counters/decrement/${cacheId}`, {
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

    // --- WebSocket ---

    subscribe(
        cacheId: string,
        onMessage: (data: CacheUpdateEvent) => void,
        onError?: (err: any) => void,
        onStatusChange?: (status: 'Connected' | 'Disconnected') => void
    ): { close: () => void } {
        return this.openSocket<CacheUpdateEvent>(
            `${this.wsUrl}/api/ws/v1/cache/${cacheId}`, onMessage, onError, onStatusChange);
    }

    subscribeCounter(
        cacheId: string,
        onMessage: (data: CounterUpdateEvent) => void,
        onError?: (err: any) => void,
        onStatusChange?: (status: 'Connected' | 'Disconnected') => void
    ): { close: () => void } {
        return this.openSocket<CounterUpdateEvent>(
            `${this.wsUrl}/api/ws/v1/counter/${cacheId}`, onMessage, onError, onStatusChange);
    }

    private openSocket<T>(
        wsUrl: string,
        onMessage: (data: T) => void,
        onError?: (err: any) => void,
        onStatusChange?: (status: 'Connected' | 'Disconnected') => void
    ): { close: () => void } {
        let ws: WebSocket | null = null;
        let isClosed = false;
        let reconnectTimeout: any = null;

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
