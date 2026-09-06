import { EmbeddedCounterCache } from './embedded_counter_cache';
import { AeronCacheClient } from './index';

// Mock AeronCacheClient
jest.mock('./index');

describe('EmbeddedCounterCache', () => {
    let mockClient: jest.Mocked<AeronCacheClient>;
    let cache: EmbeddedCounterCache;

    beforeEach(() => {
        mockClient = new AeronCacheClient('http://test', 'ws://test') as jest.Mocked<AeronCacheClient>;
        cache = new EmbeddedCounterCache(mockClient, 'test-counter');
    });

    it('delegates put to client', async () => {
        const mockResponse = { operationStatus: 'SUCCESS' };
        mockClient.putCounter.mockResolvedValue(mockResponse as any);

        const response = await cache.put('hits', 42);

        expect(mockClient.putCounter).toHaveBeenCalledWith('test-counter', 'hits', 42);
        expect(response).toEqual(mockResponse);
    });

    it('delegates increment to client', async () => {
        const mockResponse = { value: 43 };
        mockClient.incrementCounter.mockResolvedValue(mockResponse as any);

        const response = await cache.increment('hits', 1);

        expect(mockClient.incrementCounter).toHaveBeenCalledWith('test-counter', 'hits', 1);
        expect(response).toEqual(mockResponse);
    });

    it('delegates decrement to client', async () => {
        const mockResponse = { value: 41 };
        mockClient.decrementCounter.mockResolvedValue(mockResponse as any);

        await cache.decrement('hits', 1);

        expect(mockClient.decrementCounter).toHaveBeenCalledWith('test-counter', 'hits', 1);
    });

    it('delegates set to client', async () => {
        const mockResponse = { value: 100 };
        mockClient.setCounter.mockResolvedValue(mockResponse as any);

        await cache.set('hits', 100);

        expect(mockClient.setCounter).toHaveBeenCalledWith('test-counter', 'hits', 100);
    });

    it('delegates clear to client', async () => {
        const mockResponse = { operationStatus: 'SUCCESS' };
        mockClient.deleteCounterCache.mockResolvedValue(mockResponse as any);

        await cache.clear();

        expect(mockClient.deleteCounterCache).toHaveBeenCalledWith('test-counter');
    });

    it('updates local cache via subscription and returns local value', () => {
        let subscriptionCallback: any;
        const mockWs = { close: jest.fn() };

        mockClient.subscribeCounter.mockImplementation((cacheId, onMessage) => {
            subscriptionCallback = onMessage;
            return mockWs;
        });

        cache.subscribe(jest.fn());

        subscriptionCallback({
            eventType: 'ADD_ITEM',
            itemKey: 'hits',
            itemValue: 42
        });
        expect(cache.getLocal('hits')).toBe(42);

        // Zero must be honoured (not treated as missing)
        subscriptionCallback({
            eventType: 'ADD_ITEM',
            itemKey: 'hits',
            itemValue: 0
        });
        expect(cache.getLocal('hits')).toBe(0);

        subscriptionCallback({
            eventType: 'REMOVE_ITEM',
            itemKey: 'hits'
        });
        expect(cache.getLocal('hits')).toBeUndefined();
    });
});
