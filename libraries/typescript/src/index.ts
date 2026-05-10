import { EmbeddedAeronCache } from './embedded_cache';
import { 
    CreateResponse, 
    PutItemResponse, 
    GetItemResponse, 
    DeleteItemResponse, 
    DeleteCacheResponse,
    CacheUpdateEvent
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

    getCache(cacheId: string): EmbeddedAeronCache {
        return new EmbeddedAeronCache(this, cacheId);
    }

    subscribe(
        cacheId: string, 
        onMessage: (data: CacheUpdateEvent) => void, 
        onError?: (err: any) => void,
        onStatusChange?: (status: 'Connected' | 'Disconnected') => void
    ): { close: () => void } {
        let ws: WebSocket | null = null;
        let isClosed = false;
        let reconnectTimeout: any = null;
        const wsUrl = `${this.wsUrl}/api/ws/v1/cache/${cacheId}`;

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
