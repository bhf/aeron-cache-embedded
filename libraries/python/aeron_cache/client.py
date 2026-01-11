import requests
import json
import asyncio
import aiohttp
import websockets
from .embedded_cache import EmbeddedAeronCache

class AeronCacheClient:
    def __init__(self, base_url, ws_url):
        self.base_url = base_url
        self.ws_url = ws_url

    # --- Sync Operations ---

    def create_cache(self, cache_id):
        url = f"{self.base_url}/api/v1/cache"
        response = requests.post(url, json={"cacheId": cache_id})
        response.raise_for_status()
        return response.json()

    def put_item(self, cache_id, key, value):
        url = f"{self.base_url}/api/v1/cache/{cache_id}"
        response = requests.post(url, json={"cacheId": cache_id, "key": key, "value": value})
        response.raise_for_status()
        return response.json()

    def get_item(self, cache_id, key):
        url = f"{self.base_url}/api/v1/cache/{cache_id}/{key}"
        response = requests.get(url)
        response.raise_for_status()
        return response.json()

    def delete_item(self, cache_id, key):
        url = f"{self.base_url}/api/v1/cache/{cache_id}/{key}"
        response = requests.delete(url)
        response.raise_for_status()
        return response.json()
        
    def delete_cache(self, cache_id):
        url = f"{self.base_url}/api/v1/cache/{cache_id}"
        response = requests.delete(url)
        response.raise_for_status()
        return response.json()

    # --- Async Operations ---

    async def create_cache_async(self, cache_id):
        url = f"{self.base_url}/api/v1/cache"
        async with aiohttp.ClientSession() as session:
            async with session.post(url, json={"cacheId": cache_id}) as response:
                response.raise_for_status()
                return await response.json()

    async def put_item_async(self, cache_id, key, value):
        url = f"{self.base_url}/api/v1/cache/{cache_id}"
        async with aiohttp.ClientSession() as session:
            async with session.post(url, json={"cacheId": cache_id, "key": key, "value": value}) as response:
                response.raise_for_status()
                return await response.json()

    async def get_item_async(self, cache_id, key):
        url = f"{self.base_url}/api/v1/cache/{cache_id}/{key}"
        async with aiohttp.ClientSession() as session:
            async with session.get(url) as response:
                response.raise_for_status()
                return await response.json()

    async def delete_item_async(self, cache_id, key):
        url = f"{self.base_url}/api/v1/cache/{cache_id}/{key}"
        async with aiohttp.ClientSession() as session:
            async with session.delete(url) as response:
                response.raise_for_status()
                return await response.json()
    
    async def delete_cache_async(self, cache_id):
        url = f"{self.base_url}/api/v1/cache/{cache_id}"
        async with aiohttp.ClientSession() as session:
            async with session.delete(url) as response:
                response.raise_for_status()
                return await response.json()

    def get_cache(self, cache_id):
        return EmbeddedAeronCache(self, cache_id)

    # --- WebSocket ---

    async def subscribe(self, cache_id, callback):
        uri = f"{self.ws_url}/api/ws/v1/cache/{cache_id}"
        async with websockets.connect(uri) as websocket:
            async for message in websocket:
                data = json.loads(message)
                await callback(data)
