import { EmbeddedObjectCache } from './embedded_object_cache';
import { AeronCacheClient } from './index';

// Mock AeronCacheClient
jest.mock('./index');

describe('EmbeddedObjectCache', () => {
    let mockClient: jest.Mocked<AeronCacheClient>;
    let cache: EmbeddedObjectCache;

    beforeEach(() => {
        mockClient = new AeronCacheClient('http://test', 'ws://test') as jest.Mocked<AeronCacheClient>;
        cache = new EmbeddedObjectCache(mockClient, 'test-cache');
    });

    it('serializes objects on put', async () => {
        mockClient.putItem.mockResolvedValue({ operationStatus: 'SUCCESS' } as any);
        await cache.put('doc', { a: 1 });
        expect(mockClient.putItem).toHaveBeenCalledWith('test-cache', 'doc', '{"a":1}');
    });

    it('serializes fragments on patch', async () => {
        mockClient.patchItem.mockResolvedValue({ operationStatus: 'SUCCESS' } as any);
        await cache.patch('doc', { b: 2 });
        expect(mockClient.patchItem).toHaveBeenCalledWith('test-cache', 'doc', '{"b":2}');
    });

    function subscribeAndCapture(): (event: any) => void {
        let cb: any;
        mockClient.subscribe.mockImplementation((cacheId, onMessage) => {
            cb = onMessage;
            return { close: jest.fn() };
        });
        cache.subscribe(jest.fn());
        return cb;
    }

    it('stores the parsed object on ADD_ITEM', () => {
        const cb = subscribeAndCapture();
        cb({ eventType: 'ADD_ITEM', itemKey: 'doc', itemValue: '{"a":1,"b":{"c":2}}' });
        expect(cache.getLocal('doc')).toEqual({ a: 1, b: { c: 2 } });
    });

    it('deep-merges PATCH_ITEM instead of overwriting', () => {
        const cb = subscribeAndCapture();
        cb({ eventType: 'ADD_ITEM', itemKey: 'doc', itemValue: '{"a":1,"b":{"c":2}}' });
        cb({ eventType: 'PATCH_ITEM', itemKey: 'doc', itemValue: '{"b":{"d":3}}' });
        expect(cache.getLocal('doc')).toEqual({ a: 1, b: { c: 2, d: 3 } });
    });

    it('deletes a field when the delta value is null (RFC 7386)', () => {
        const cb = subscribeAndCapture();
        cb({ eventType: 'ADD_ITEM', itemKey: 'doc', itemValue: '{"a":1,"b":2}' });
        cb({ eventType: 'PATCH_ITEM', itemKey: 'doc', itemValue: '{"b":null}' });
        expect(cache.getLocal('doc')).toEqual({ a: 1 });
    });

    it('patch on an absent key starts from the delta', () => {
        const cb = subscribeAndCapture();
        cb({ eventType: 'PATCH_ITEM', itemKey: 'doc', itemValue: '{"a":1}' });
        expect(cache.getLocal('doc')).toEqual({ a: 1 });
    });

    it('replaces scalars and arrays rather than merging them', () => {
        const cb = subscribeAndCapture();
        cb({ eventType: 'ADD_ITEM', itemKey: 'doc', itemValue: '{"n":1,"list":[1,2,3]}' });
        cb({ eventType: 'PATCH_ITEM', itemKey: 'doc', itemValue: '{"n":9,"list":[4]}' });
        expect(cache.getLocal('doc')).toEqual({ n: 9, list: [4] });
    });

    it('handles REMOVE_ITEM and CLEAR_CACHE', () => {
        const cb = subscribeAndCapture();
        cb({ eventType: 'ADD_ITEM', itemKey: 'doc', itemValue: '{"a":1}' });
        cb({ eventType: 'REMOVE_ITEM', itemKey: 'doc' });
        expect(cache.getLocal('doc')).toBeUndefined();

        cb({ eventType: 'ADD_ITEM', itemKey: 'doc', itemValue: '{"a":1}' });
        cb({ eventType: 'CLEAR_CACHE' });
        expect(cache.getLocal('doc')).toBeUndefined();
    });

    it('deepMerge does not mutate its inputs', () => {
        const target = { a: 1, b: { c: 2 } };
        const patch = { b: { d: 3 } };
        const merged = EmbeddedObjectCache.deepMerge(target, patch);
        expect(merged).toEqual({ a: 1, b: { c: 2, d: 3 } });
        expect(target).toEqual({ a: 1, b: { c: 2 } });
    });
});
