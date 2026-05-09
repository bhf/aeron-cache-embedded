import { EmbeddedAeronCache } from './embedded_cache';
import { AeronCacheClient } from './index';

// Mock AeronCacheClient
jest.mock('./index');

describe('EmbeddedAeronCache', () => {
    let mockClient: jest.Mocked<AeronCacheClient>;
    let cache: EmbeddedAeronCache;

    beforeEach(() => {
        mockClient = new AeronCacheClient('http://test', 'ws://test') as jest.Mocked<AeronCacheClient>;
        cache = new EmbeddedAeronCache(mockClient, 'test-cache');
    });

    it('delegates put to client', async () => {
        const mockResponse = { operationStatus: 'SUCCESS' };
        mockClient.putItem.mockResolvedValue(mockResponse as any);

        const response = await cache.put('key1', 'val1');

        expect(mockClient.putItem).toHaveBeenCalledWith('test-cache', 'key1', 'val1');
        expect(response).toEqual(mockResponse);
    });

    it('delegates delete to client', async () => {
        const mockResponse = { operationStatus: 'SUCCESS' };
        mockClient.deleteItem.mockResolvedValue(mockResponse as any);

        const response = await cache.delete('key1');

        expect(mockClient.deleteItem).toHaveBeenCalledWith('test-cache', 'key1');
        expect(response).toEqual(mockResponse);
    });

    it('delegates clear to client', async () => {
        const mockResponse = { operationStatus: 'SUCCESS' };
        mockClient.deleteCache.mockResolvedValue(mockResponse as any);

        const response = await cache.clear();

        expect(mockClient.deleteCache).toHaveBeenCalledWith('test-cache');
        expect(response).toEqual(mockResponse);
    });

    it('updates local cache via subscription and returns local value', () => {
        let subscriptionCallback: any;
        const mockWs = { close: jest.fn() };
        
        mockClient.subscribe.mockImplementation((cacheId, onMessage) => {
            subscriptionCallback = onMessage;
            return mockWs;
        });

        // Initialize subscription
        cache.subscribe(jest.fn());

        // Simulate incoming web socket message adding an item
        subscriptionCallback({
            eventType: 'ADD_ITEM',
            itemKey: 'my-key',
            itemValue: 'my-value'
        });

        // Check local read
        expect(cache.getLocal('my-key')).toEqual('my-value');

        // Simulate item removal
        subscriptionCallback({
            eventType: 'REMOVE_ITEM',
            itemKey: 'my-key'
        });

        // Check local read (should be removed)
        expect(cache.getLocal('my-key')).toBeUndefined();
    });
});
