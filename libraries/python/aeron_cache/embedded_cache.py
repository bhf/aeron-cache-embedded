
class EmbeddedAeronCache:
    def __init__(self, client, cache_id):
        self.client = client
        self.cache_id = cache_id
        self.local_cache = {}

    def get_local(self, key):
        return self.local_cache.get(key)

    def put(self, key, value):
        return self.client.put_item(self.cache_id, key, value)

    def get(self, key):
        return self.client.get_item(self.cache_id, key)

    def remove(self, key):
        return self.client.delete_item(self.cache_id, key)

    def clear(self):
        return self.client.delete_cache(self.cache_id)

    async def put_async(self, key, value):
        return await self.client.put_item_async(self.cache_id, key, value)

    async def get_async(self, key):
        return await self.client.get_item_async(self.cache_id, key)

    async def remove_async(self, key):
        return await self.client.delete_item_async(self.cache_id, key)

    async def clear_async(self):
        return await self.client.delete_cache_async(self.cache_id)

    async def subscribe(self, callback):
        async def wrapped_callback(data):
            self._update_local_cache(data)
            await callback(data)
        return await self.client.subscribe(self.cache_id, wrapped_callback)

    def _update_local_cache(self, event):
        event_type = event.get('eventType')
        if event_type == 'ADD_ITEM':
            self.local_cache[event.get('itemKey')] = event.get('itemValue')
        elif event_type == 'REMOVE_ITEM':
            self.local_cache.pop(event.get('itemKey'), None)
        elif event_type in ['CLEAR_CACHE', 'DELETE_CACHE']:
            self.local_cache.clear()
