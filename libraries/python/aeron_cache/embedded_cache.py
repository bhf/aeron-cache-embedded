
from .models import (
    PutItemResponse, 
    GetItemResponse, 
    DeleteItemResponse, 
    DeleteCacheResponse,
    CacheUpdateEvent
)

class EmbeddedAeronCache:
    def __init__(self, client, cache_id):
        self.client = client
        self.cache_id = cache_id
        self.local_cache = {}

    def get_local(self, key):
        return self.local_cache.get(key)

    def put(self, key, value) -> PutItemResponse:
        return self.client.put_item(self.cache_id, key, value)

    def put_timed(self, key, value, ttl) -> PutItemResponse:
        return self.client.put_timed_item(self.cache_id, key, value, ttl)

    def get(self, key) -> GetItemResponse:
        return self.client.get_item(self.cache_id, key)

    def remove(self, key) -> DeleteItemResponse:
        return self.client.delete_item(self.cache_id, key)

    def clear(self) -> DeleteCacheResponse:
        return self.client.delete_cache(self.cache_id)

    async def put_async(self, key, value) -> PutItemResponse:
        return await self.client.put_item_async(self.cache_id, key, value)

    async def put_timed_async(self, key, value, ttl) -> PutItemResponse:
        return await self.client.put_timed_item_async(self.cache_id, key, value, ttl)

    async def get_async(self, key) -> GetItemResponse:
        return await self.client.get_item_async(self.cache_id, key)

    async def remove_async(self, key) -> DeleteItemResponse:
        return await self.client.delete_item_async(self.cache_id, key)

    async def clear_async(self) -> DeleteCacheResponse:
        return await self.client.delete_cache_async(self.cache_id)

    async def subscribe(self, callback, hydrate: bool = False, keys=None, mode=None):
        if mode is not None and str(getattr(mode, 'value', mode)).lower() == 'patch':
            raise ValueError(
                "EmbeddedAeronCache does not support patch-mode subscriptions: PATCH_ITEM deltas "
                "cannot be merged into opaque string values. Use EmbeddedObjectCache instead."
            )

        async def wrapped_callback(event: CacheUpdateEvent):
            self._update_local_cache(event)
            if callback:
                import asyncio
                if asyncio.iscoroutinefunction(callback):
                    await callback(event)
                else:
                    callback(event)
        return await self.client.subscribe(self.cache_id, wrapped_callback, hydrate=hydrate, keys=keys, mode=mode)

    def _update_local_cache(self, event: CacheUpdateEvent):
        event_type = event.eventType
        if event_type == 'ADD_ITEM':
            if event.itemKey and event.itemValue:
                self.local_cache[event.itemKey] = event.itemValue
        elif event_type == 'PATCH_ITEM':
            # Intentionally ignored: PATCH_ITEM carries only the changed fields (a delta), so applying it
            # to this string mirror would overwrite the full stored value with the fragment and lose the
            # untouched fields. Use EmbeddedObjectCache, which deep-merges deltas, for patch mode.
            pass
        elif event_type == 'REMOVE_ITEM':
            if event.itemKey:
                self.local_cache.pop(event.itemKey, None)
        elif event_type in ['CLEAR_CACHE', 'DELETE_CACHE']:
            self.local_cache.clear()
