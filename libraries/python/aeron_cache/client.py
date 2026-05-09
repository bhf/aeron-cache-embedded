import requests
import json
import asyncio
import aiohttp
import websockets
from .embedded_cache import EmbeddedAeronCache
from .models import (
    CreateResponse, 
    PutItemResponse, 
    GetItemResponse, 
    DeleteItemResponse, 
    DeleteCacheResponse,
    CacheUpdateEvent
)

class AeronCacheClient:
    def __init__(self, base_url, ws_url):
        self.base_url = base_url
        self.ws_url = ws_url

    # --- Sync Operations ---

    def create_cache(self, cache_id) -> CreateResponse:
        url = f"{self.base_url}/api/v1/cache"
        response = requests.post(url, json={"cacheId": cache_id})
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
             response.raise_for_status()
        data = response.json()
        return CreateResponse(
            cacheId=data.get('cacheId'),
            operationStatus=data.get('operationStatus')
        )

    def put_item(self, cache_id, key, value) -> PutItemResponse:
        url = f"{self.base_url}/api/v1/cache/{cache_id}"
        response = requests.post(url, json={"cacheId": cache_id, "key": key, "value": value})
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
             response.raise_for_status()
        data = response.json()
        return PutItemResponse(
            cacheId=data.get('cacheId'),
            key=data.get('key'),
            operationStatus=data.get('operationStatus')
        )

    def get_item(self, cache_id, key) -> GetItemResponse:
        url = f"{self.base_url}/api/v1/cache/{cache_id}/{key}"
        response = requests.get(url)
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
             response.raise_for_status()
        data = response.json()
        return GetItemResponse(
            cacheId=data.get('cacheId'),
            key=data.get('key'),
            value=data.get('value'),
            operationStatus=data.get('operationStatus')
        )

    def delete_item(self, cache_id, key) -> DeleteItemResponse:
        url = f"{self.base_url}/api/v1/cache/{cache_id}/{key}"
        response = requests.delete(url)
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
             response.raise_for_status()
        data = response.json()
        return DeleteItemResponse(
            cacheId=data.get('cacheId'),
            key=data.get('key'),
            operationStatus=data.get('operationStatus')
        )
        
    def delete_cache(self, cache_id) -> DeleteCacheResponse:
        url = f"{self.base_url}/api/v1/cache/{cache_id}"
        response = requests.delete(url)
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
             response.raise_for_status()
        data = response.json()
        return DeleteCacheResponse(
            cacheId=data.get('cacheId'),
            operationStatus=data.get('operationStatus')
        )

    # --- Async Operations ---

    async def create_cache_async(self, cache_id) -> CreateResponse:
        url = f"{self.base_url}/api/v1/cache"
        async with aiohttp.ClientSession() as session:
            async with session.post(url, json={"cacheId": cache_id}) as response:
                if response.status >= 400 and response.status not in [400, 401, 404]:
                    response.raise_for_status()
                data = await response.json()
                return CreateResponse(
                    cacheId=data.get('cacheId'),
                    operationStatus=data.get('operationStatus')
                )

    async def put_item_async(self, cache_id, key, value) -> PutItemResponse:
        url = f"{self.base_url}/api/v1/cache/{cache_id}"
        async with aiohttp.ClientSession() as session:
            async with session.post(url, json={"cacheId": cache_id, "key": key, "value": value}) as response:
                if response.status >= 400 and response.status not in [400, 401, 404]:
                    response.raise_for_status()
                data = await response.json()
                return PutItemResponse(
                    cacheId=data.get('cacheId'),
                    key=data.get('key'),
                    operationStatus=data.get('operationStatus')
                )

    async def get_item_async(self, cache_id, key) -> GetItemResponse:
        url = f"{self.base_url}/api/v1/cache/{cache_id}/{key}"
        async with aiohttp.ClientSession() as session:
            async with session.get(url) as response:
                if response.status >= 400 and response.status not in [400, 401, 404]:
                    response.raise_for_status()
                data = await response.json()
                return GetItemResponse(
                    cacheId=data.get('cacheId'),
                    key=data.get('key'),
                    value=data.get('value'),
                    operationStatus=data.get('operationStatus')
                )

    async def delete_item_async(self, cache_id, key) -> DeleteItemResponse:
        url = f"{self.base_url}/api/v1/cache/{cache_id}/{key}"
        async with aiohttp.ClientSession() as session:
            async with session.delete(url) as response:
                if response.status >= 400 and response.status not in [400, 401, 404]:
                    response.raise_for_status()
                data = await response.json()
                return DeleteItemResponse(
                    cacheId=data.get('cacheId'),
                    key=data.get('key'),
                    operationStatus=data.get('operationStatus')
                )
        
    async def delete_cache_async(self, cache_id) -> DeleteCacheResponse:
        url = f"{self.base_url}/api/v1/cache/{cache_id}"
        async with aiohttp.ClientSession() as session:
            async with session.delete(url) as response:
                if response.status >= 400 and response.status not in [400, 401, 404]:
                    response.raise_for_status()
                data = await response.json()
                return DeleteCacheResponse(
                    cacheId=data.get('cacheId'),
                    operationStatus=data.get('operationStatus')
                )

    # --- WebSocket ---

    async def subscribe(self, cache_id, callback):
        uri = f"{self.ws_url}/api/ws/v1/cache/{cache_id}"
        while True:
            try:
                async with websockets.connect(uri) as websocket:
                    async for message in websocket:
                        data = json.loads(message)
                        event = CacheUpdateEvent.from_dict(data)
                        await callback(event)
            except (websockets.ConnectionClosed, Exception):
                await asyncio.sleep(5)
