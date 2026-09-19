
import json

from .models import (
    PutItemResponse,
    GetItemResponse,
    DeleteItemResponse,
    DeleteCacheResponse,
    PatchItemResponse,
    CacheUpdateEvent,
)


def _deep_merge(target: dict, patch: dict) -> dict:
    """RFC 7386 (JSON Merge Patch) deep-merge of ``patch`` into ``target``, returning a new dict.

    Nested objects merge recursively; scalars and lists replace; a ``None`` field deletes the field.
    """
    result = dict(target)
    for name, value in patch.items():
        if value is None:
            result.pop(name, None)
        elif isinstance(value, dict):
            existing = result.get(name)
            base = existing if isinstance(existing, dict) else {}
            result[name] = _deep_merge(base, value)
        else:
            result[name] = value
    return result


class EmbeddedObjectCache:
    """A cache view whose values are structured JSON objects rather than opaque strings.

    The key difference from :class:`EmbeddedAeronCache` is how ``PATCH_ITEM`` events are applied: instead of
    overwriting the entry with the delta, the delta is deep-merged into the stored object using RFC 7386
    (JSON Merge Patch) semantics -- nested objects merge recursively, scalars and lists are replaced, and a
    ``None`` field in the delta deletes that field. This matches the server-side ``patch_item`` deep-merge,
    so a patch-mode subscription (which streams only the changed fields) reconstructs the full object locally
    without losing untouched fields.

    Values are held as ``dict`` (parsed JSON). ``put``/``patch`` accept a ``dict`` and serialize it.
    """

    def __init__(self, client, cache_id):
        self.client = client
        self.cache_id = cache_id
        self.local_cache = {}

    def get_local(self, key):
        """The locally mirrored object (a ``dict``) for ``key``, or ``None`` if absent."""
        return self.local_cache.get(key)

    def put(self, key, value: dict) -> PutItemResponse:
        return self.client.put_item(self.cache_id, key, json.dumps(value))

    def put_timed(self, key, value: dict, ttl) -> PutItemResponse:
        return self.client.put_timed_item(self.cache_id, key, json.dumps(value), ttl)

    def patch(self, key, fragment: dict) -> PatchItemResponse:
        """Deep-merge a JSON fragment into the stored object (RFC 7386). A ``None`` field deletes it."""
        return self.client.patch_item(self.cache_id, key, json.dumps(fragment))

    def get(self, key) -> GetItemResponse:
        return self.client.get_item(self.cache_id, key)

    def remove(self, key) -> DeleteItemResponse:
        return self.client.delete_item(self.cache_id, key)

    def clear(self) -> DeleteCacheResponse:
        return self.client.delete_cache(self.cache_id)

    async def put_async(self, key, value: dict) -> PutItemResponse:
        return await self.client.put_item_async(self.cache_id, key, json.dumps(value))

    async def put_timed_async(self, key, value: dict, ttl) -> PutItemResponse:
        return await self.client.put_timed_item_async(self.cache_id, key, json.dumps(value), ttl)

    async def patch_async(self, key, fragment: dict) -> PatchItemResponse:
        return await self.client.patch_item_async(self.cache_id, key, json.dumps(fragment))

    async def get_async(self, key) -> GetItemResponse:
        return await self.client.get_item_async(self.cache_id, key)

    async def remove_async(self, key) -> DeleteItemResponse:
        return await self.client.delete_item_async(self.cache_id, key)

    async def clear_async(self) -> DeleteCacheResponse:
        return await self.client.delete_cache_async(self.cache_id)

    async def subscribe(self, callback, hydrate: bool = False, keys=None, mode=None):
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
                self.local_cache[event.itemKey] = self._parse_object(event.itemValue)
        elif event_type == 'PATCH_ITEM':
            if event.itemKey and event.itemValue:
                delta = self._parse_object(event.itemValue)
                existing = self.local_cache.get(event.itemKey) or {}
                self.local_cache[event.itemKey] = _deep_merge(existing, delta)
        elif event_type == 'REMOVE_ITEM':
            if event.itemKey:
                self.local_cache.pop(event.itemKey, None)
        elif event_type in ['CLEAR_CACHE', 'DELETE_CACHE']:
            self.local_cache.clear()

    @staticmethod
    def _parse_object(value: str) -> dict:
        parsed = json.loads(value)
        if not isinstance(parsed, dict):
            raise ValueError(f"EmbeddedObjectCache values must be JSON objects, got: {value}")
        return parsed
