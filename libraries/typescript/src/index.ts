import { EmbeddedAeronCache } from './embedded_cache';

export { EmbeddedAeronCache };

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

    async createCache(cacheId: string): Promise<any> {
        const response = await fetch(`${this.baseUrl}/api/v1/cache`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ cacheId })
        });
        return this.handleResponse(response);
    }

    async putItem(cacheId: string, key: string, value: string): Promise<any> {
        const response = await fetch(`${this.baseUrl}/api/v1/cache/${cacheId}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ cacheId, key, value })
        });
        return this.handleResponse(response);
    }

    async getItem(cacheId: string, key: string): Promise<any> {
        const response = await fetch(`${this.baseUrl}/api/v1/cache/${cacheId}/${key}`);
        return this.handleResponse(response);
    }

    async deleteItem(cacheId: string, key: string): Promise<any> {
        const response = await fetch(`${this.baseUrl}/api/v1/cache/${cacheId}/${key}`, {
            method: 'DELETE'
        });
        return this.handleResponse(response);
    }

    async deleteCache(cacheId: string): Promise<any> {
        const response = await fetch(`${this.baseUrl}/api/v1/cache/${cacheId}`, {
            method: 'DELETE'
        });
        return this.handleResponse(response);
    }

    getCache(cacheId: string): EmbeddedAeronCache {
        return new EmbeddedAeronCache(this, cacheId);
    }

    subscribe(cacheId: string, onMessage: (data: any) => void, onError?: (err: any) => void): WebSocket {
        const ws = new WebSocket(`${this.wsUrl}/api/ws/v1/cache/${cacheId}`);
        ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                onMessage(data);
            } catch (e) {
                if (onError) onError(e);
            }
        };
        if (onError) ws.onerror = onError;
        return ws;
    }

    private async handleResponse(response: Response): Promise<any> {
        if (!response.ok) {
            throw new Error(`HTTP Error: ${response.status} ${response.statusText}`);
        }
        return response.json();
    }
}
