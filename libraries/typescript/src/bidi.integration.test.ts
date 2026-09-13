import { AeronBidiClient } from './bidi';
import { CacheUpdateEvent } from './models';

const baseUrl = process.env.AERON_CACHE_BASE_URL;
let wsUrl = process.env.AERON_CACHE_WS_URL;

const shouldRun = !!baseUrl;
if (shouldRun && !wsUrl) {
    wsUrl = baseUrl!.replace('http://', 'ws://').replace('https://', 'wss://');
}

const rid = () => Math.random().toString(36).substring(2, 10);

(shouldRun ? describe : describe.skip)('AeronBidiClient Integration', () => {
    let client: AeronBidiClient;

    beforeAll(async () => {
        client = new AeronBidiClient(wsUrl!);
        await client.connect();
    });

    afterAll(async () => {
        await client.close();
    });

    it('cache lifecycle', async () => {
        const cacheId = `bidi-cache-${rid()}`;
        expect((await client.createCache(cacheId)).cacheId).toBe(cacheId);
        expect((await client.putItem(cacheId, 'k1', 'v1')).operationStatus).toBe('SUCCESS');
        expect((await client.getItem(cacheId, 'k1')).value).toBe('v1');

        await client.putItem(cacheId, 'doc', '{"a":1}');
        await client.patchItem(cacheId, 'doc', '{"b":2}');
        const doc = (await client.getItem(cacheId, 'doc')).value;
        expect(doc).toContain('"a":1');
        expect(doc).toContain('"b":2');

        const items = (await client.getCacheItems(cacheId)).items;
        expect(new Set(items.map((i) => i.key))).toEqual(new Set(['k1', 'doc']));

        expect((await client.deleteItem(cacheId, 'k1')).operationStatus).toBe('SUCCESS');
        expect((await client.clearCache(cacheId)).operationStatus).toBe('SUCCESS');
        await client.deleteCache(cacheId);
    });

    it('counter lifecycle', async () => {
        const cacheId = `bidi-counter-${rid()}`;
        await client.createCounterCache(cacheId);
        await client.putCounter(cacheId, 'hits', 10);
        expect((await client.incrementCounter(cacheId, 'hits', 5)).value).toBe(15);
        expect((await client.decrementCounter(cacheId, 'hits', 3)).value).toBe(12);
        expect((await client.setCounter(cacheId, 'hits', 100)).value).toBe(100);
        expect((await client.getCounter(cacheId, 'hits')).value).toBe(100);

        await client.putCounter(cacheId, 'misses', 7);
        const got = await client.getCounterItems(cacheId);
        const map: Record<string, number> = {};
        for (const i of got.items) map[i.key] = i.value;
        expect(map).toEqual({ hits: 100, misses: 7 });

        await client.clearCounterCache(cacheId);
        await client.deleteCounterCache(cacheId);
    });

    it('getStats returns per-cache entries', async () => {
        const cacheId = `bidi-stats-${rid()}`;
        await client.createCache(cacheId);
        await client.putItem(cacheId, 'k', 'v');
        const stats = await client.getStats();
        expect(Array.isArray(stats)).toBe(true);
        const entry = stats.find((s) => s.cacheId === cacheId);
        expect(entry).toBeDefined();
        expect(entry!.size).toBeGreaterThanOrEqual(1);
        await client.deleteCache(cacheId);
    });

    it('cancel-item-removal keeps a timed item', async () => {
        const cacheId = `bidi-cancel-${rid()}`;
        await client.createCache(cacheId);
        await client.putTimedItem(cacheId, 'keep', 'val', 2000);
        expect((await client.cancelItemRemoval(cacheId, 'keep')).key).toBe('keep');
        await new Promise((r) => setTimeout(r, 3000));
        expect((await client.getItem(cacheId, 'keep')).value).toBe('val');
        await client.deleteCache(cacheId);
    }, 10000);

    it('subscription receives ADD_ITEM', async () => {
        const cacheId = `bidi-sub-${rid()}`;
        await client.createCache(cacheId);

        const received: CacheUpdateEvent[] = [];
        const done = new Promise<void>((resolve) => {
            client
                .subscribe(cacheId, (ev) => {
                    if (ev.eventType === 'ADD_ITEM' && ev.itemKey === 'sk') {
                        received.push(ev);
                        resolve();
                    }
                })
                .then(() => client.putItem(cacheId, 'sk', 'sv'));
        });

        await Promise.race([
            done,
            new Promise((_, reject) => setTimeout(() => reject(new Error('no stream update')), 5000))
        ]);
        expect(received[0].itemValue).toBe('sv');
        await client.deleteCache(cacheId);
    }, 10000);
});
