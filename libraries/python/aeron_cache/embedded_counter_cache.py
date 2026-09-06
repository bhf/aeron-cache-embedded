
from .models import (
    PutItemResponse,
    DeleteItemResponse,
    DeleteCacheResponse,
    CounterResponse,
    CounterUpdateEvent
)

class EmbeddedCounterCache:
    def __init__(self, client, cache_id):
        self.client = client
        self.cache_id = cache_id
        self.local_cache = {}

    def get_local(self, key):
        return self.local_cache.get(key)

    def put(self, key, value) -> PutItemResponse:
        return self.client.put_counter(self.cache_id, key, value)

    def put_timed(self, key, value, ttl) -> PutItemResponse:
        return self.client.put_timed_counter(self.cache_id, key, value, ttl)

    def get(self, key) -> CounterResponse:
        return self.client.get_counter(self.cache_id, key)

    def increment(self, key, amount) -> CounterResponse:
        return self.client.increment_counter(self.cache_id, key, amount)

    def decrement(self, key, amount) -> CounterResponse:
        return self.client.decrement_counter(self.cache_id, key, amount)

    def set(self, key, value) -> CounterResponse:
        return self.client.set_counter(self.cache_id, key, value)

    def remove(self, key) -> DeleteItemResponse:
        return self.client.delete_counter(self.cache_id, key)

    def clear(self) -> DeleteCacheResponse:
        return self.client.delete_counter_cache(self.cache_id)

    async def put_async(self, key, value) -> PutItemResponse:
        return await self.client.put_counter_async(self.cache_id, key, value)

    async def put_timed_async(self, key, value, ttl) -> PutItemResponse:
        return await self.client.put_timed_counter_async(self.cache_id, key, value, ttl)

    async def get_async(self, key) -> CounterResponse:
        return await self.client.get_counter_async(self.cache_id, key)

    async def increment_async(self, key, amount) -> CounterResponse:
        return await self.client.increment_counter_async(self.cache_id, key, amount)

    async def decrement_async(self, key, amount) -> CounterResponse:
        return await self.client.decrement_counter_async(self.cache_id, key, amount)

    async def set_async(self, key, value) -> CounterResponse:
        return await self.client.set_counter_async(self.cache_id, key, value)

    async def remove_async(self, key) -> DeleteItemResponse:
        return await self.client.delete_counter_async(self.cache_id, key)

    async def clear_async(self) -> DeleteCacheResponse:
        return await self.client.delete_counter_cache_async(self.cache_id)

    async def subscribe(self, callback, hydrate: bool = False):
        async def wrapped_callback(event: CounterUpdateEvent):
            self._update_local_cache(event)
            if callback:
                import asyncio
                if asyncio.iscoroutinefunction(callback):
                    await callback(event)
                else:
                    callback(event)
        return await self.client.subscribe_counter(self.cache_id, wrapped_callback, hydrate=hydrate)

    def _update_local_cache(self, event: CounterUpdateEvent):
        event_type = event.eventType
        if event_type == 'ADD_ITEM':
            # `is not None` so that a counter value of 0 is stored, not skipped.
            if event.itemKey is not None and event.itemValue is not None:
                self.local_cache[event.itemKey] = event.itemValue
        elif event_type == 'REMOVE_ITEM':
            if event.itemKey is not None:
                self.local_cache.pop(event.itemKey, None)
        elif event_type in ['CLEAR_CACHE', 'DELETE_CACHE']:
            self.local_cache.clear()
