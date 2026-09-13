import { AeronCacheClient } from './index';

describe('AeronCacheClient counters', () => {
    let client: AeronCacheClient;

    beforeEach(() => {
        client = new AeronCacheClient('http://localhost:7070', 'ws://localhost:7071');
        global.fetch = jest.fn();
    });

    const mockJson = (body: any, status = 200, ok = true) => {
        (global.fetch as jest.Mock).mockResolvedValue({ ok, status, json: async () => body });
    };

    it('should create counter cache', async () => {
        const mockResponse = { cacheId: 'test-counter', operationStatus: 'SUCCESS' };
        mockJson(mockResponse);

        const response = await client.createCounterCache('test-counter');

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ cacheId: 'test-counter' })
        });
        expect(response).toEqual(mockResponse);
    });

    it('should put counter', async () => {
        mockJson({ cacheId: 'test-counter', key: 'hits', operationStatus: 'SUCCESS' });

        await client.putCounter('test-counter', 'hits', 42);

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/test-counter', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key: 'hits', value: 42 })
        });
    });

    it('should put timed counter', async () => {
        mockJson({ cacheId: 'test-counter', key: 'hits', operationStatus: 'SUCCESS' });

        await client.putTimedCounter('test-counter', 'hits', 42, 1000);

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/timed/test-counter', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key: 'hits', value: 42, ttl: 1000 })
        });
    });

    it('should get counter', async () => {
        mockJson({ cacheId: 'test-counter', key: 'hits', value: 42, operationStatus: 'SUCCESS' });

        const response = await client.getCounter('test-counter', 'hits');

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/test-counter/hits');
        expect(response.value).toBe(42);
    });

    it('should increment counter', async () => {
        mockJson({ cacheId: 'test-counter', key: 'hits', value: 43, operationStatus: 'SUCCESS' });

        const response = await client.incrementCounter('test-counter', 'hits', 1);

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/increment/test-counter', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key: 'hits', amount: 1 })
        });
        expect(response.value).toBe(43);
    });

    it('should decrement counter', async () => {
        mockJson({ cacheId: 'test-counter', key: 'hits', value: 41, operationStatus: 'SUCCESS' });

        const response = await client.decrementCounter('test-counter', 'hits', 1);

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/decrement/test-counter', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key: 'hits', amount: 1 })
        });
        expect(response.value).toBe(41);
    });

    it('should set counter', async () => {
        mockJson({ cacheId: 'test-counter', key: 'hits', value: 100, operationStatus: 'SUCCESS' });

        const response = await client.setCounter('test-counter', 'hits', 100);

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/set/test-counter', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key: 'hits', value: 100 })
        });
        expect(response.value).toBe(100);
    });

    it('should delete counter and cache', async () => {
        mockJson({ cacheId: 'test-counter', key: 'hits', operationStatus: 'SUCCESS' });
        await client.deleteCounter('test-counter', 'hits');
        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/test-counter/hits', { method: 'DELETE' });

        mockJson({ cacheId: 'test-counter', operationStatus: 'SUCCESS' });
        await client.deleteCounterCache('test-counter');
        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/test-counter', { method: 'DELETE' });
    });

    it('should perform bulk operations with counters', async () => {
        const mockResponse = {
            requestId: 'req-1',
            operationResponses: [
                { requestId: 'op-1', status: 'SUCCESS', cacheId: 'test-counter' },
                { requestId: 'op-2', status: 'SUCCESS', cacheId: 'test-counter', key: 'hits', value: '5' }
            ]
        };
        mockJson(mockResponse);

        const request = {
            requestId: 'req-1',
            operations: [
                { operationType: 'CREATE_COUNTER_CACHE' as const, requestId: 'op-1', cacheId: 'test-counter' },
                { operationType: 'INCREMENT_COUNTER' as const, requestId: 'op-2', cacheId: 'test-counter', key: 'hits', counterValue: 5 }
            ]
        };
        const response = await client.bulkOps(request);

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache/bulkops', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(request)
        });
        expect(response.operationResponses).toHaveLength(2);
    });

    it('should throw on 500 error', async () => {
        (global.fetch as jest.Mock).mockResolvedValue({ ok: false, status: 500, statusText: 'Internal Server Error' });
        await expect(client.getCounter('test-counter', 'hits')).rejects.toThrow('HTTP Error: 500');
    });

    it('should get counter items', async () => {
        mockJson({
            cacheId: 'test-counter',
            operationStatus: 'SUCCESS',
            items: [{ key: 'hits', value: 42 }, { key: 'misses', value: 7 }]
        });

        const response = await client.getCounterItems('test-counter');

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/test-counter');
        expect(response.cacheId).toBe('test-counter');
        expect(response.items).toHaveLength(2);
        expect(response.items[0].key).toBe('hits');
        expect(response.items[0].value).toBe(42);
    });

    it('should clear counter cache', async () => {
        mockJson({ cacheId: 'test-counter', operationStatus: 'SUCCESS' });

        const response = await client.clearCounterCache('test-counter');

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/test-counter', { method: 'PATCH' });
        expect(response.operationStatus).toBe('SUCCESS');
    });

    it('should cancel counter item removal', async () => {
        mockJson({ cacheId: 'test-counter', key: 'hits', operationStatus: 'SUCCESS' });

        const response = await client.cancelCounterItemRemoval('test-counter', 'hits');

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/test-counter/hits/cancel-removal', { method: 'POST' });
        expect(response.cacheId).toBe('test-counter');
        expect(response.key).toBe('hits');
    });

    it('should get counter caches', async () => {
        mockJson([{ cacheId: 'cc1', itemCount: 4 }]);

        const response = await client.getCounterCaches();

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters-caches');
        expect(response).toHaveLength(1);
        expect(response[0].cacheId).toBe('cc1');
        expect(response[0].itemCount).toBe(4);
    });

    it('should get counter stats', async () => {
        mockJson({ totalOpsCount: 3, totalCachesCount: 1, totalItemsCount: 4, errorCount: 0 });

        const response = await client.getCounterStats();

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters-stats');
        expect(response.totalItemsCount).toBe(4);
        expect(response.totalCachesCount).toBe(1);
    });
});
