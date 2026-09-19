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
            let wsResult: { close: () => void } | undefined;

            const timeout = setTimeout(() => {
                if (wsResult) wsResult.close();
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
                            if (wsResult) wsResult.close();
                            resolve();
                        } catch (e) {
                            reject(e);
                        }
                    }, 2000);
                }
            };

            wsResult = embedded.subscribe(() => {}, () => {}, onStatusChange, true);
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

    test('get_timers returns a pending cache timer', async () => {
        const cacheId = `it-timers-${crypto.randomUUID()}`;
        await client.createCache(cacheId);
        await client.putTimedItem(cacheId, 'timed', 'val', 600000);

        const resp = await client.getTimers();
        expect(resp.operationStatus).toBe('SUCCESS');
        expect(Array.isArray(resp.timers)).toBe(true);
        const timer = resp.timers.find(t => t.cacheId === cacheId && t.key === 'timed');
        expect(timer).toBeDefined();
        expect(timer!.timerType).toBe('CACHE');
        expect(timer!.deadline).toBeGreaterThan(0);

        await client.deleteCache(cacheId);
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

    it('should perform counter operations', async () => {
        const cacheId = `it-counter-${Math.random().toString(36).substring(7)}`;

        const createResp = await client.createCounterCache(cacheId);
        expect(createResp.cacheId).toBe(cacheId);

        const counters = client.getCounterCache(cacheId);

        const putResp = await counters.put('hits', 10);
        expect(putResp.key).toBe('hits');

        expect((await counters.increment('hits', 5)).value).toBe(15);
        expect((await counters.decrement('hits', 3)).value).toBe(12);
        expect((await counters.set('hits', 100)).value).toBe(100);
        expect((await counters.get('hits')).value).toBe(100);

        await counters.delete('hits');
        await counters.clear();
    });

    it('should handle counter websocket subscriptions', async () => {
        const cacheId = `it-counter-ws-${Math.random().toString(36).substring(7)}`;
        await client.createCounterCache(cacheId);
        const counters = client.getCounterCache(cacheId);

        return new Promise<void>((resolve, reject) => {
            const timeout = setTimeout(() => {
                ws.close();
                reject(new Error('Counter websocket event not received within timeout'));
            }, 10000);

            let opened = false;

            const onMessage = (event: any) => {
                if (event.eventType === 'ADD_ITEM' && event.itemKey === 'ws-counter') {
                    clearTimeout(timeout);
                    try {
                        expect(counters.getLocal('ws-counter')).toBe(7);
                        ws.close();
                        resolve();
                    } catch (e) {
                        reject(e);
                    }
                }
            };

            const onStatusChange = (status: 'Connected' | 'Disconnected') => {
                if (status === 'Connected' && !opened) {
                    opened = true;
                    counters.put('ws-counter', 7).catch(err => {
                        clearTimeout(timeout);
                        ws.close();
                        reject(err);
                    });
                }
            };

            const ws = counters.subscribe(onMessage, () => {}, onStatusChange);
        });
    }, 15000);

    test('patch_item deep-merges document', async () => {
        const cacheId = `it-patch-${Date.now()}`;
        await client.createCache(cacheId);

        await client.putItem(cacheId, 'doc', '{"a":1}');
        const patchResp = await client.patchItem(cacheId, 'doc', '{"b":2}');
        expect(patchResp.cacheId).toBe(cacheId);
        expect(patchResp.key).toBe('doc');

        const getResp = await client.getItem(cacheId, 'doc');
        // deep-merge should retain both fields
        expect(getResp.value).toContain('"a"');
        expect(getResp.value).toContain('"b"');
    });

    test('cancel_item_removal keeps a timed item', async () => {
        const cacheId = `it-cancel-${Date.now()}`;
        await client.createCache(cacheId);

        await client.putTimedItem(cacheId, 'keep', 'val', 2000);
        const cancelResp = await client.cancelItemRemoval(cacheId, 'keep');
        expect(cancelResp.cacheId).toBe(cacheId);
        expect(cancelResp.key).toBe('keep');

        await new Promise(resolve => setTimeout(resolve, 3000));
        const getResp = await client.getItem(cacheId, 'keep');
        expect(getResp.value).toBe('val');
    }, 10000);

    test('get_caches_and_stats behaves correctly', async () => {
        const cacheId = `it-caches-${Date.now()}`;
        await client.createCache(cacheId);
        await client.putItem(cacheId, 'k', 'v');

        const caches = await client.getCaches();
        expect(caches.some(c => c.cacheId === cacheId)).toBeTruthy();

        const stats = await client.getStats();
        expect(stats.totalCachesCount).toBeGreaterThanOrEqual(1);
        expect(stats.totalItemsCount).toBeGreaterThanOrEqual(1);
    });

    test('get_counter_items_and_clear behaves correctly', async () => {
        const cacheId = `it-citems-${Date.now()}`;
        await client.createCounterCache(cacheId);
        await client.putCounter(cacheId, 'a', 1);
        await client.putCounter(cacheId, 'b', 2);

        const resp = await client.getCounterItems(cacheId);
        expect(resp.cacheId).toBe(cacheId);
        expect(resp.items.length).toBe(2);

        const clearResp = await client.clearCounterCache(cacheId);
        expect(clearResp.operationStatus).toBe('SUCCESS');
    });

    test('cancel_counter_item_removal keeps a timed counter', async () => {
        const cacheId = `it-ccancel-${Date.now()}`;
        await client.createCounterCache(cacheId);

        await client.putTimedCounter(cacheId, 'keep', 9, 2000);
        const cancelResp = await client.cancelCounterItemRemoval(cacheId, 'keep');
        expect(cancelResp.cacheId).toBe(cacheId);
        expect(cancelResp.key).toBe('keep');

        await new Promise(resolve => setTimeout(resolve, 3000));
        expect((await client.getCounter(cacheId, 'keep')).value).toBe(9);
    }, 10000);

    test('get_counter_caches_and_stats behaves correctly', async () => {
        const cacheId = `it-ccaches-${Date.now()}`;
        await client.createCounterCache(cacheId);
        await client.putCounter(cacheId, 'k', 1);

        const caches = await client.getCounterCaches();
        expect(caches.some(c => c.cacheId === cacheId)).toBeTruthy();

        const stats = await client.getCounterStats();
        expect(stats.totalCachesCount).toBeGreaterThanOrEqual(1);
    });

    it('should filter websocket events by key', async () => {
        const cacheId = `it-ws-keys-${Math.random().toString(36).substring(7)}`;
        await client.createCache(cacheId);

        const received: string[] = [];

        return new Promise<void>((resolve, reject) => {
            let opened = false;

            const timeout = setTimeout(() => {
                ws.close();
                try {
                    expect(received).toContain('key1');
                    expect(received).not.toContain('key2');
                    resolve();
                } catch (e) {
                    reject(e);
                }
            }, 4000);

            const onMessage = (event: any) => {
                if (event.eventType === 'ADD_ITEM' && event.itemKey) {
                    received.push(event.itemKey);
                }
            };

            const onStatusChange = (status: 'Connected' | 'Disconnected') => {
                if (status === 'Connected' && !opened) {
                    opened = true;
                    setTimeout(() => {
                        Promise.all([
                            client.putItem(cacheId, 'key1', 'v1'),
                            client.putItem(cacheId, 'key2', 'v2')
                        ]).catch(err => {
                            clearTimeout(timeout);
                            ws.close();
                            reject(err);
                        });
                    }, 1000);
                }
            };

            // Subscribe filtered to only "key1".
            const ws = client.subscribe(cacheId, onMessage, () => {}, onStatusChange, false, 'key1');
        });
    }, 15000);

    it('should receive PATCH_ITEM events in patch mode', async () => {
        const cacheId = `it-ws-patch-${Math.random().toString(36).substring(7)}`;
        await client.createCache(cacheId);
        await client.putItem(cacheId, 'doc', '{"a":1}');

        return new Promise<void>((resolve, reject) => {
            let opened = false;

            const timeout = setTimeout(() => {
                ws.close();
                reject(new Error('PATCH_ITEM event not received within timeout'));
            }, 8000);

            const onMessage = (event: any) => {
                if (event.eventType === 'PATCH_ITEM' && event.itemKey === 'doc') {
                    clearTimeout(timeout);
                    ws.close();
                    resolve();
                }
            };

            const onStatusChange = (status: 'Connected' | 'Disconnected') => {
                if (status === 'Connected' && !opened) {
                    opened = true;
                    setTimeout(() => {
                        client.patchItem(cacheId, 'doc', '{"b":2}').catch(err => {
                            clearTimeout(timeout);
                            ws.close();
                            reject(err);
                        });
                    }, 1000);
                }
            };

            const ws = client.subscribe(cacheId, onMessage, () => {}, onStatusChange, false, undefined, 'patch');
        });
    }, 15000);
});
