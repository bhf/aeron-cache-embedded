import { EmbeddedAeronCache } from './embedded_cache';
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
    BulkCacheOpsResponse
} from './models';

export { EmbeddedAeronCache };
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

    subscribe(
        cacheIds: string, 
        onMessage: (data: CacheUpdateEvent) => void, 
        onError?: (err: any) => void,
        onStatusChange?: (status: 'Connected' | 'Disconnected') => void,
        hydrate: boolean = false
    ): { close: () => void } {
        let ws: WebSocket | null = null;
        let isClosed = false;
        let reconnectTimeout: any = null;
        let finalWsUrl = this.wsUrl;
        
        const prefix = hydrate ? 
            (cacheIds.includes(',') ? '/api/ws/v1/caches/hydrate' : '/api/ws/v1/cache/hydrate') :
            (cacheIds.includes(',') ? '/api/ws/v1/caches' : '/api/ws/v1/cache');

        const wsUrl = `${finalWsUrl.replace(/\/$/, '')}${prefix}/${cacheIds}`;

        const connect = () => {
            if (isClosed) return;
            
            ws = new WebSocket(wsUrl);

            ws.onopen = () => {
                if (onStatusChange) onStatusChange('Connected');
            };
            
            ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data) as CacheUpdateEvent;
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
