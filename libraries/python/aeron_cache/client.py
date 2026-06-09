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
    GetCacheResponse,
    ClearCacheResponse,
    CacheUpdateEvent,
    BulkCacheOpsRequest,
    BulkCacheOpsResponse,
    BulkOperationType
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
        response = requests.post(url, json={"key": key, "value": value})
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
             response.raise_for_status()
        data = response.json()
        return PutItemResponse(
            cacheId=data.get('cacheId'),
            key=data.get('key'),
            operationStatus=data.get('operationStatus')
        )

    def put_timed_item(self, cache_id, key, value, ttl) -> PutItemResponse:
        url = f"{self.base_url}/api/v1/cache/timed/{cache_id}"
        response = requests.post(url, json={"key": key, "value": value, "ttl": ttl})
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

    def bulk_ops(self, request: BulkCacheOpsRequest) -> BulkCacheOpsResponse:
        url = f"{self.base_url}/api/v1/cache/bulkops"
        payload = {
            "requestId": request.requestId,
            "operations": [
                {k: (v.value if hasattr(v, 'value') else v) 
                 for k, v in op.__dict__.items() if v is not None}
                for op in request.operations
            ]
        }
        response = requests.post(url, json=payload)
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
             response.raise_for_status()
        return BulkCacheOpsResponse.from_dict(response.json())

    # --- WebSocket ---

    async def subscribe(self, cache_ids: str, on_message, hydrate: bool = False):
        """
        Subscribe to updates for one or more caches.
        :param cache_ids: Comma-separated list of cache IDs.
        :param on_message: Callback for incoming messages.
        :param hydrate: Whether to request initial hydration.
        """
        prefix = "/api/ws/v1/cache/hydrate" if hydrate else "/api/ws/v1/cache"
        if "," in cache_ids:
            prefix = "/api/ws/v1/caches/hydrate" if hydrate else "/api/ws/v1/caches"
        
        uri = f"{self.ws_url.rstrip('/')}{prefix}/{cache_ids}"
        
        while True:
            try:
                async with websockets.connect(uri) as websocket:
                    async for message in websocket:
                        data = json.loads(message)
                        event = CacheUpdateEvent.from_dict(data)
                        if asyncio.iscoroutinefunction(on_message):
                            await on_message(event)
                        else:
                            on_message(event)
            except Exception as e:
                print(f"WebSocket error: {e}. Reconnecting in 5s...")
                await asyncio.sleep(5)
        # Convert request to dict, handling Enum and Optional fields
        payload = {
            "requestId": request.requestId,
            "operations": [
                {k: (v.value if isinstance(v, BulkOperationType) else v) 
                 for k, v in op.__dict__.items() if v is not None}
                for op in request.operations
            ]
        }
        response = requests.post(url, json=payload)
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
             response.raise_for_status()
        return BulkCacheOpsResponse.from_dict(response.json())

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
            async with session.post(url, json={"key": key, "value": value}) as response:
                if response.status >= 400 and response.status not in [400, 401, 404]:
                    response.raise_for_status()
                data = await response.json()
                return PutItemResponse(
                    cacheId=data.get('cacheId'),
                    key=data.get('key'),
                    operationStatus=data.get('operationStatus')
                )

    async def put_timed_item_async(self, cache_id, key, value, ttl) -> PutItemResponse:
        url = f"{self.base_url}/api/v1/cache/timed/{cache_id}"
        async with aiohttp.ClientSession() as session:
            async with session.post(url, json={"key": key, "value": value, "ttl": ttl}) as response:
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
        
    def get_cache_items(self, cache_id) -> GetCacheResponse:
        url = f"{self.base_url}/api/v1/cache/{cache_id}"
        response = requests.get(url)
        return GetCacheResponse.from_dict(response.json())

    def clear_cache(self, cache_id) -> ClearCacheResponse:
        url = f"{self.base_url}/api/v1/cache/{cache_id}"
        response = requests.patch(url)
        return ClearCacheResponse(**response.json())

    async def get_cache_async(self, cache_id) -> GetCacheResponse:
        url = f"{self.base_url}/api/v1/cache/{cache_id}"
        async with aiohttp.ClientSession() as session:
            async with session.get(url) as response:
                return GetCacheResponse.from_dict(await response.json())

    async def clear_cache_async(self, cache_id) -> ClearCacheResponse:
        url = f"{self.base_url}/api/v1/cache/{cache_id}"
        async with aiohttp.ClientSession() as session:
            async with session.patch(url) as response:
                return ClearCacheResponse(**await response.json())

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

    async def bulk_ops_async(self, request: BulkCacheOpsRequest) -> BulkCacheOpsResponse:
        url = f"{self.base_url}/api/v1/cache/bulkops"
        payload = {
            "requestId": request.requestId,
            "operations": [
                {k: (v.value if isinstance(v, BulkOperationType) else v) 
                 for k, v in op.__dict__.items() if v is not None}
                for op in request.operations
            ]
        }
        async with aiohttp.ClientSession() as session:
            async with session.post(url, json=payload) as response:
                if response.status >= 400 and response.status not in [400, 401, 404]:
                    response.raise_for_status()
                data = await response.json()
                return BulkCacheOpsResponse.from_dict(data)

    # --- WebSocket ---

    def get_cache(self, cache_id: str) -> EmbeddedAeronCache:
        return EmbeddedAeronCache(self, cache_id)

    async def subscribe(self, cache_ids: str, on_message, hydrate: bool = False):
        """
        Subscribe to updates for one or more caches.
        :param cache_ids: Comma-separated list of cache IDs.
        :param on_message: Callback for incoming messages.
        :param hydrate: Whether to request initial hydration.
        """
        prefix = "/api/ws/v1/cache/hydrate" if hydrate else "/api/ws/v1/cache"
        if "," in cache_ids:
            prefix = "/api/ws/v1/caches/hydrate" if hydrate else "/api/ws/v1/caches"
        
        uri = f"{self.ws_url.rstrip('/')}{prefix}/{cache_ids}"
        
        while True:
            try:
                async with websockets.connect(uri) as websocket:
                    async for message in websocket:
                        data = json.loads(message)
                        event = CacheUpdateEvent.from_dict(data)
                        if asyncio.iscoroutinefunction(on_message):
                            await on_message(event)
                        else:
                            on_message(event)
            except Exception as e:
                print(f"WebSocket error: {e}. Reconnecting in 5s...")
                await asyncio.sleep(5)
