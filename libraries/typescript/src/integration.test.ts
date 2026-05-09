import { AeronCacheClient } from './index';

const baseUrl = process.env.AERON_CACHE_BASE_URL;
let wsUrl = process.env.AERON_CACHE_WS_URL;

const shouldRun = !!baseUrl;
if (shouldRun && !wsUrl) {
    wsUrl = baseUrl.replace('http://', 'ws://').replace('https://', 'wss://');
}

(shouldRun ? describe : describe.skip)('AeronCacheClient Integration', () => {
    let client: AeronCacheClient;

    beforeAll(() => {
        client = new AeronCacheClient(baseUrl!, wsUrl!);
    });

    it('should perform cache operations', async () => {
        const cacheId = `it-cache-${Math.random().toString(36).substring(7)}`;

        const createResp = await client.createCache(cacheId);
        expect(createResp).toBeDefined();
        expect(createResp.cacheId).toBe(cacheId);

        const embedded = client.getCache(cacheId);

        // Put an item
        const putResp = await embedded.put('key1', 'val1');
        expect(putResp).toBeDefined();
        expect(putResp.key).toBe('key1');

        // Get the item
        const getResp = await embedded.get('key1');
        expect(getResp).toBeDefined();
        expect(getResp.value).toBe('val1');

        // Remove the item
        const delResp = await embedded.delete('key1');
        expect(delResp).toBeDefined();

        // Get the item again, should handle 404 cleanly
        const getResp2 = await embedded.get('key1');
        expect(getResp2).toBeDefined();
        expect(getResp2.operationStatus === 'UNKNOWN_KEY' || getResp2.value == null).toBeTruthy();
    });

    it('should handle websocket subscriptions', async () => {
        const cacheId = `it-ws-${Math.random().toString(36).substring(7)}`;
        await client.createCache(cacheId);
        const embedded = client.getCache(cacheId);

        return new Promise<void>((resolve, reject) => {
            const timeout = setTimeout(() => {
                ws.close();
                reject(new Error('Websocket event not received within timeout'));
            }, 5000);

            let openedTriggered = false;

            const onMessage = (event: any) => {
                if (event.eventType === 'ADD_ITEM' && event.itemKey === 'ws-key') {
                    clearTimeout(timeout);
                    
                    try {
                        expect(embedded.getLocal('ws-key')).toBe('ws-val');
                        ws.close();
                        resolve();
                    } catch (e) {
                        reject(e);
                    }
                }
            };

            const onError = (err: any) => {
                console.error("Websocket error", err);
            };

            const onStatusChange = (status: 'Connected' | 'Disconnected') => {
                if (status === 'Connected' && !openedTriggered) {
                    openedTriggered = true;
                    // Trigger cache update which should reflect in websocket
                    embedded.put('ws-key', 'ws-val').catch(err => {
                        clearTimeout(timeout);
                        ws.close();
                        reject(err);
                    });
                }
            };

            const ws = embedded.subscribe(onMessage, onError, onStatusChange);
        });
    });
});
