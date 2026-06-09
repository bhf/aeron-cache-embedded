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
            }, 10000);

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
                console.error("Websocket error in TS suite. WS URL is:", wsUrl);
            };

            const onStatusChange = (status: 'Connected' | 'Disconnected') => {
                if (status === 'Connected' && !openedTriggered) {
                    openedTriggered = true;
                    // Trigger cache update which should reflect in websocket
                    setTimeout(() => {
                        embedded.put('ws-key', 'ws-val').catch(err => {
                            clearTimeout(timeout);
                            ws.close();
                            reject(err);
                        });
                    }, 1000);
                }
            };

            const ws = embedded.subscribe(onMessage, onError, onStatusChange);
        });
    }, 15000); // Give Jest 15s before failing

    it('should hydrate existing data when subscribing', async () => {
        const cacheId = `it-hydrate-${Math.random().toString(36).substring(7)}`;
        await client.createCache(cacheId);
        
        const preFill = client.getCache(cacheId);
        await preFill.put('hydrate-key', 'hydrate-val');

        const embedded = client.getCache(cacheId);
        
        return new Promise<void>((resolve, reject) => {
            const timeout = setTimeout(() => {
                embedded.unsubscribe();
                reject(new Error('Hydration event not received within timeout'));
            }, 5000);

            const onStatusChange = (status: 'Connected' | 'Disconnected') => {
                if (status === 'Connected') {
                    // Give it a moment to process hydration
                    setTimeout(() => {
                        try {
                            const localVal = embedded.getLocal('hydrate-key');
                            expect(localVal).toBe('hydrate-val');
                            clearTimeout(timeout);
                            embedded.unsubscribe();
                            resolve();
                        } catch (e) {
                            reject(e);
                        }
                    }, 2000);
                }
            };

            embedded.subscribe(() => {}, () => {}, onStatusChange, true);
        });
    });

    test('get_and_clear_cache behaves correctly', async () => {
        const cacheId = `it-cache2-${Date.now()}`;
        
        await client.createCache(cacheId);
        await client.putItem(cacheId, 'key1', 'val1');
        await client.putItem(cacheId, 'key2', 'val2');

        const getResp = await client.getCacheItems(cacheId);
        expect(getResp.items.length).toBe(2);

        const clearResp = await client.clearCache(cacheId);
        expect(clearResp.operationStatus).toBe('SUCCESS');

        const getResp2 = await client.getCacheItems(cacheId);
        expect(getResp2.items.length).toBe(0);
    });

    test('bulk_operations behaves correctly', async () => {
        const cacheId = `it-bulk-${Date.now()}`;
        const requestId = `req-${Date.now()}`;

        const request = {
            requestId,
            operations: [
                {
                    operationType: 'CREATE_CACHE' as const,
                    requestId: 'op-1',
                    cacheId
                },
                {
                    operationType: 'ADD_ITEM' as const,
                    requestId: 'op-2',
                    cacheId,
                    key: 'bulk-key',
                    value: 'bulk-val'
                },
                {
                    operationType: 'GET_ITEM' as const,
                    requestId: 'op-3',
                    cacheId,
                    key: 'bulk-key'
                }
            ]
        };

        const response = await client.bulkOps(request);
        expect(response.requestId).toBe(requestId);
        expect(response.operationResponses.length).toBe(3);

        expect(response.operationResponses[2].requestId).toBe('op-3');
        expect(response.operationResponses[2].value).toBe('bulk-val');
    });

    it("should handle putTimedItem correctly", async () => {
        const cacheId = `it-timed-${Date.now()}`;
        await client.createCache(cacheId);
        const embedded = client.getCache(cacheId);

        // Put a timed item with 2 second TTL (2000 ms)
        const putResp = await embedded.putTimed("timed-key", "timed-val", 2000);
        expect(putResp.key).toBe("timed-key");

        // Get immediately - should exist
        const getResp = await embedded.get("timed-key");
        expect(getResp.value).toBe("timed-val");

        // Wait for TTL to expire (3 seconds)
        await new Promise(resolve => setTimeout(resolve, 3000));

        // Get again - should be gone
        const getResp2 = await embedded.get("timed-key");
        expect(getResp2.operationStatus === "UNKNOWN_KEY" || !getResp2.value).toBeTruthy();
    });
});
