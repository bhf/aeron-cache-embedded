import { AeronCacheClient } from './index';

describe('AeronCacheClient counters', () => {
    let client: AeronCacheClient;

    beforeEach(() => {
        client = new AeronCacheClient('http://localhost:7070', 'ws://localhost:7071');
        global.fetch = jest.fn();
    });

    it('should create counter cache', async () => {
        const mockResponse = { cacheId: 'test-counter', operationStatus: 'SUCCESS' };
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: true,
            status: 200,
            json: async () => mockResponse
        });

        const response = await client.createCounterCache('test-counter');

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ cacheId: 'test-counter' })
        });
        expect(response).toEqual(mockResponse);
    });

    it('should put counter', async () => {
        const mockResponse = { cacheId: 'test-counter', key: 'hits', operationStatus: 'SUCCESS' };
        (global.fetch as jest.Mock).mockResolvedValue({ ok: true, status: 200, json: async () => mockResponse });

        const response = await client.putCounter('test-counter', 'hits', 42);

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/test-counter', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key: 'hits', value: 42 })
        });
        expect(response).toEqual(mockResponse);
    });

    it('should get counter', async () => {
        const mockResponse = { cacheId: 'test-counter', key: 'hits', value: 42, operationStatus: 'SUCCESS' };
        (global.fetch as jest.Mock).mockResolvedValue({ ok: true, status: 200, json: async () => mockResponse });

        const response = await client.getCounter('test-counter', 'hits');

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/test-counter/hits');
        expect(response.value).toBe(42);
    });

    it('should increment counter', async () => {
        const mockResponse = { cacheId: 'test-counter', key: 'hits', value: 43, operationStatus: 'SUCCESS' };
        (global.fetch as jest.Mock).mockResolvedValue({ ok: true, status: 200, json: async () => mockResponse });

        const response = await client.incrementCounter('test-counter', 'hits', 1);

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/increment/test-counter', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key: 'hits', amount: 1 })
        });
        expect(response.value).toBe(43);
    });

    it('should decrement counter', async () => {
        const mockResponse = { cacheId: 'test-counter', key: 'hits', value: 41, operationStatus: 'SUCCESS' };
        (global.fetch as jest.Mock).mockResolvedValue({ ok: true, status: 200, json: async () => mockResponse });

        const response = await client.decrementCounter('test-counter', 'hits', 1);

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/decrement/test-counter', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key: 'hits', amount: 1 })
        });
        expect(response.value).toBe(41);
    });

    it('should set counter', async () => {
        const mockResponse = { cacheId: 'test-counter', key: 'hits', value: 100, operationStatus: 'SUCCESS' };
        (global.fetch as jest.Mock).mockResolvedValue({ ok: true, status: 200, json: async () => mockResponse });

        const response = await client.setCounter('test-counter', 'hits', 100);

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/set/test-counter', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key: 'hits', value: 100 })
        });
        expect(response.value).toBe(100);
    });

    it('should delete counter', async () => {
        const mockResponse = { cacheId: 'test-counter', key: 'hits', operationStatus: 'SUCCESS' };
        (global.fetch as jest.Mock).mockResolvedValue({ ok: true, status: 200, json: async () => mockResponse });

        const response = await client.deleteCounter('test-counter', 'hits');

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/test-counter/hits', {
            method: 'DELETE'
        });
        expect(response).toEqual(mockResponse);
    });

    it('should delete counter cache', async () => {
        const mockResponse = { cacheId: 'test-counter', operationStatus: 'SUCCESS' };
        (global.fetch as jest.Mock).mockResolvedValue({ ok: true, status: 200, json: async () => mockResponse });

        const response = await client.deleteCounterCache('test-counter');

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/counters/test-counter', {
            method: 'DELETE'
        });
        expect(response).toEqual(mockResponse);
    });

    it('should perform bulk operations', async () => {
        const mockResponse = {
            requestId: 'req-1',
            operationResponses: [
                { requestId: 'op-1', status: 'SUCCESS', cacheId: 'test-counter' },
                { requestId: 'op-2', status: 'SUCCESS', cacheId: 'test-counter', key: 'hits', value: '5' }
            ]
        };
        (global.fetch as jest.Mock).mockResolvedValue({ ok: true, status: 200, json: async () => mockResponse });

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
});
