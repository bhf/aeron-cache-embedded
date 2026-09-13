import { AeronCacheClient } from './index';

describe('AeronCacheClient', () => {
    let client: AeronCacheClient;

    beforeEach(() => {
        client = new AeronCacheClient('http://localhost:7070', 'ws://localhost:7071');
        // Mock global fetch
        global.fetch = jest.fn();
    });

    it('should create cache', async () => {
        const mockResponse = { cacheId: 'test-cache', status: 'CREATED' };
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: true,
            status: 200,
            json: async () => mockResponse
        });

        const response = await client.createCache('test-cache');
        
        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ cacheId: 'test-cache' })
        });
        expect(response).toEqual(mockResponse);
    });

    it('should put item', async () => {
        const mockResponse = { success: true };
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: true,
            status: 200,
            json: async () => mockResponse
        });

        const response = await client.putItem('test-cache', 'my-key', 'my-value');
        
        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache/test-cache', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key: 'my-key', value: 'my-value' })
        });
        expect(response).toEqual(mockResponse);
    });

    it('should get item', async () => {
        const mockResponse = { key: 'my-key', value: 'my-value' };
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: true,
            status: 200,
            json: async () => mockResponse
        });

        const response = await client.getItem('test-cache', 'my-key');
        
        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache/test-cache/my-key');
        expect(response).toEqual(mockResponse);
    });

    it('should delete item', async () => {
        const mockResponse = { success: true };
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: true,
            status: 200,
            json: async () => mockResponse
        });

        const response = await client.deleteItem('test-cache', 'my-key');
        
        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache/test-cache/my-key', {
            method: 'DELETE'
        });
        expect(response).toEqual(mockResponse);
    });

    it('should delete cache', async () => {
        const mockResponse = { success: true };
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: true,
            status: 200,
            json: async () => mockResponse
        });

        const response = await client.deleteCache('test-cache');
        
        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache/test-cache', {
            method: 'DELETE'
        });
        expect(response).toEqual(mockResponse);
    });

    it('should perform bulk operations', async () => {
        const mockResponse = { requestId: 'req-1', operationResponses: [] };
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: true,
            status: 200,
            json: async () => mockResponse
        });

        const request = { requestId: 'req-1', operations: [] };
        const response = await client.bulkOps(request);
        
        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache/bulkops', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(request)
        });
        expect(response).toEqual(mockResponse);
    });

    it('should throw error on 500 status', async () => {
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: false,
            status: 500,
            statusText: 'Internal Server Error'
        });

        await expect(client.getItem('test-cache', 'my-key')).rejects.toThrow('HTTP Error: 500 Internal Server Error');
    });

    it('should allow business logic HTTP errors (400, 404)', async () => {
        const mockResponse = { operationStatus: 'UNKNOWN_KEY' };
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: false,
            status: 404, // 404 is in allowed list and should return json instead of throwing
            json: async () => mockResponse
        });

        const response = await client.getItem('test-cache', 'non-existent-key');
        expect(response).toEqual(mockResponse);
    });

    test('getCacheItems makes correct request', async () => {
        global.fetch = jest.fn().mockResolvedValue({
            ok: true,
            json: async () => ({ cacheId: 'test-cache-get', operationStatus: 'SUCCESS', items: [{key: 'k1', value: 'v1'}] })
        });

        const response = await client.getCacheItems('test-cache-get');
        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache/test-cache-get');
        expect(response.cacheId).toBe('test-cache-get');
        expect(response.items.length).toBe(1);
        expect(response.items[0].key).toBe('k1');
    });

    test('clearCache makes correct request', async () => {
        global.fetch = jest.fn().mockResolvedValue({
            ok: true,
            json: async () => ({ cacheId: 'test-cache-clear', operationStatus: 'SUCCESS' })
        });

        const response = await client.clearCache('test-cache-clear');
        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache/test-cache-clear', { method: 'PATCH' });
        expect(response.operationStatus).toBe('SUCCESS');
    });

    it('should put timed item', async () => {
        const mockResponse = { cacheId: 'test-cache', key: 'k1', operationStatus: 'SUCCESS' };
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: true,
            status: 200,
            json: async () => mockResponse
        });

        const response = await client.putTimedItem('test-cache', 'k1', 'v1', 1000);
        
        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache/timed/test-cache', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key: 'k1', value: 'v1', ttl: 1000 })
        });
        expect(response).toEqual(mockResponse);
    });

    test('clearCache makes correct request', async () => {
        global.fetch = jest.fn().mockResolvedValue({
            ok: true,
            json: async () => ({ cacheId: 'test-cache-clear', operationStatus: 'SUCCESS' })
        });

        const response = await client.clearCache('test-cache-clear');
        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache/test-cache-clear', { method: 'PATCH' });
        expect(response.operationStatus).toBe('SUCCESS');
    });

    it('should patch item', async () => {
        const mockResponse = { cacheId: 'test-cache', key: 'doc', operationStatus: 'SUCCESS' };
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: true,
            status: 200,
            json: async () => mockResponse
        });

        const response = await client.patchItem('test-cache', 'doc', '{"b":2}');

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache/test-cache/doc', {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ value: '{"b":2}' })
        });
        expect(response).toEqual(mockResponse);
    });

    it('should cancel item removal', async () => {
        const mockResponse = { cacheId: 'test-cache', key: 'keep', operationStatus: 'SUCCESS' };
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: true,
            status: 200,
            json: async () => mockResponse
        });

        const response = await client.cancelItemRemoval('test-cache', 'keep');

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/cache/test-cache/keep/cancel-removal', {
            method: 'POST'
        });
        expect(response).toEqual(mockResponse);
    });

    it('should get caches', async () => {
        const mockResponse = [{ cacheId: 'c1', itemCount: 3 }, { cacheId: 'c2', itemCount: 0 }];
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: true,
            status: 200,
            json: async () => mockResponse
        });

        const response = await client.getCaches();

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/caches');
        expect(response).toHaveLength(2);
        expect(response[0].cacheId).toBe('c1');
        expect(response[0].itemCount).toBe(3);
    });

    it('should get stats', async () => {
        const mockResponse = { totalOpsCount: 10, totalCachesCount: 2, totalItemsCount: 5, errorCount: 1 };
        (global.fetch as jest.Mock).mockResolvedValue({
            ok: true,
            status: 200,
            json: async () => mockResponse
        });

        const response = await client.getStats();

        expect(global.fetch).toHaveBeenCalledWith('http://localhost:7070/api/v1/stats');
        expect(response).toEqual(mockResponse);
    });

    describe('wsQuery', () => {
        it('returns empty string when neither param is provided', () => {
            expect(AeronCacheClient.wsQuery()).toBe('');
            expect(AeronCacheClient.wsQuery(undefined, undefined)).toBe('');
        });

        it('builds keys-only query', () => {
            expect(AeronCacheClient.wsQuery('key1')).toBe('?keys=key1');
        });

        it('builds mode-only query', () => {
            expect(AeronCacheClient.wsQuery(undefined, 'patch')).toBe('?mode=patch');
        });

        it('builds combined keys and mode query', () => {
            expect(AeronCacheClient.wsQuery('cacheA:key1,key2', 'patch'))
                .toBe(`?keys=${encodeURIComponent('cacheA:key1,key2')}&mode=patch`);
        });

        it('URL-encodes each value', () => {
            expect(AeronCacheClient.wsQuery('a b:c/d', 'full'))
                .toBe(`?keys=${encodeURIComponent('a b:c/d')}&mode=full`);
        });
    });
});
