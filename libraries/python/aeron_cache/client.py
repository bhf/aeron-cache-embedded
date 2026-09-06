import requests
import json
import asyncio
import aiohttp
import websockets
from .embedded_cache import EmbeddedAeronCache
from .embedded_counter_cache import EmbeddedCounterCache
from .models import (
    CreateResponse,
    PutItemResponse,
    GetItemResponse,
    DeleteItemResponse,
    DeleteCacheResponse,
    CacheUpdateEvent,
    CounterResponse,
    CounterUpdateEvent,
    BulkCacheOpsRequest,
    BulkCacheOpsResponse
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

    # --- Counter Operations (Sync) ---

    def create_counter_cache(self, cache_id) -> CreateResponse:
        url = f"{self.base_url}/api/v1/counters/"
        response = requests.post(url, json={"cacheId": cache_id})
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
            response.raise_for_status()
        data = response.json()
        return CreateResponse(
            cacheId=data.get('cacheId'),
            operationStatus=data.get('operationStatus')
        )

    def put_counter(self, cache_id, key, value) -> PutItemResponse:
        url = f"{self.base_url}/api/v1/counters/{cache_id}"
        response = requests.post(url, json={"key": key, "value": value})
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
            response.raise_for_status()
        data = response.json()
        return PutItemResponse(
            cacheId=data.get('cacheId'),
            key=data.get('key'),
            operationStatus=data.get('operationStatus')
        )

    def get_counter(self, cache_id, key) -> CounterResponse:
        url = f"{self.base_url}/api/v1/counters/{cache_id}/{key}"
        response = requests.get(url)
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
            response.raise_for_status()
        data = response.json()
        return CounterResponse(
            cacheId=data.get('cacheId'),
            key=data.get('key'),
            value=data.get('value'),
            operationStatus=data.get('operationStatus')
        )

    def delete_counter(self, cache_id, key) -> DeleteItemResponse:
        url = f"{self.base_url}/api/v1/counters/{cache_id}/{key}"
        response = requests.delete(url)
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
            response.raise_for_status()
        data = response.json()
        return DeleteItemResponse(
            cacheId=data.get('cacheId'),
            key=data.get('key'),
            operationStatus=data.get('operationStatus')
        )

    def delete_counter_cache(self, cache_id) -> DeleteCacheResponse:
        url = f"{self.base_url}/api/v1/counters/{cache_id}"
        response = requests.delete(url)
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
            response.raise_for_status()
        data = response.json()
        return DeleteCacheResponse(
            cacheId=data.get('cacheId'),
            operationStatus=data.get('operationStatus')
        )

    def increment_counter(self, cache_id, key, amount) -> CounterResponse:
        return self._counter_op("increment", cache_id, {"key": key, "amount": amount})

    def decrement_counter(self, cache_id, key, amount) -> CounterResponse:
        return self._counter_op("decrement", cache_id, {"key": key, "amount": amount})

    def set_counter(self, cache_id, key, value) -> CounterResponse:
        return self._counter_op("set", cache_id, {"key": key, "value": value})

    def _counter_op(self, op, cache_id, body) -> CounterResponse:
        url = f"{self.base_url}/api/v1/counters/{op}/{cache_id}"
        response = requests.post(url, json=body)
        if response.status_code >= 400 and response.status_code not in [400, 401, 404]:
            response.raise_for_status()
        data = response.json()
        return CounterResponse(
            cacheId=data.get('cacheId'),
            key=data.get('key'),
            value=data.get('value'),
            operationStatus=data.get('operationStatus')
        )

    def bulk_ops(self, request: BulkCacheOpsRequest) -> BulkCacheOpsResponse:
        url = f"{self.base_url}/api/v1/cache/bulkops"
        response = requests.post(url, json=request.to_dict())
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

    # --- Counter Operations (Async) ---

    async def create_counter_cache_async(self, cache_id) -> CreateResponse:
        url = f"{self.base_url}/api/v1/counters/"
        async with aiohttp.ClientSession() as session:
            async with session.post(url, json={"cacheId": cache_id}) as response:
                if response.status >= 400 and response.status not in [400, 401, 404]:
                    response.raise_for_status()
                data = await response.json()
                return CreateResponse(
                    cacheId=data.get('cacheId'),
                    operationStatus=data.get('operationStatus')
                )

    async def put_counter_async(self, cache_id, key, value) -> PutItemResponse:
        url = f"{self.base_url}/api/v1/counters/{cache_id}"
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

    async def get_counter_async(self, cache_id, key) -> CounterResponse:
        url = f"{self.base_url}/api/v1/counters/{cache_id}/{key}"
        async with aiohttp.ClientSession() as session:
            async with session.get(url) as response:
                if response.status >= 400 and response.status not in [400, 401, 404]:
                    response.raise_for_status()
                data = await response.json()
                return CounterResponse(
                    cacheId=data.get('cacheId'),
                    key=data.get('key'),
                    value=data.get('value'),
                    operationStatus=data.get('operationStatus')
                )

    async def delete_counter_async(self, cache_id, key) -> DeleteItemResponse:
        url = f"{self.base_url}/api/v1/counters/{cache_id}/{key}"
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

    async def delete_counter_cache_async(self, cache_id) -> DeleteCacheResponse:
        url = f"{self.base_url}/api/v1/counters/{cache_id}"
        async with aiohttp.ClientSession() as session:
            async with session.delete(url) as response:
                if response.status >= 400 and response.status not in [400, 401, 404]:
                    response.raise_for_status()
                data = await response.json()
                return DeleteCacheResponse(
                    cacheId=data.get('cacheId'),
                    operationStatus=data.get('operationStatus')
                )

    async def increment_counter_async(self, cache_id, key, amount) -> CounterResponse:
        return await self._counter_op_async("increment", cache_id, {"key": key, "amount": amount})

    async def decrement_counter_async(self, cache_id, key, amount) -> CounterResponse:
        return await self._counter_op_async("decrement", cache_id, {"key": key, "amount": amount})

    async def set_counter_async(self, cache_id, key, value) -> CounterResponse:
        return await self._counter_op_async("set", cache_id, {"key": key, "value": value})

    async def _counter_op_async(self, op, cache_id, body) -> CounterResponse:
        url = f"{self.base_url}/api/v1/counters/{op}/{cache_id}"
        async with aiohttp.ClientSession() as session:
            async with session.post(url, json=body) as response:
                if response.status >= 400 and response.status not in [400, 401, 404]:
                    response.raise_for_status()
                data = await response.json()
                return CounterResponse(
                    cacheId=data.get('cacheId'),
                    key=data.get('key'),
                    value=data.get('value'),
                    operationStatus=data.get('operationStatus')
                )

    async def bulk_ops_async(self, request: BulkCacheOpsRequest) -> BulkCacheOpsResponse:
        url = f"{self.base_url}/api/v1/cache/bulkops"
        async with aiohttp.ClientSession() as session:
            async with session.post(url, json=request.to_dict()) as response:
                if response.status >= 400 and response.status not in [400, 401, 404]:
                    response.raise_for_status()
                return BulkCacheOpsResponse.from_dict(await response.json())

    # --- WebSocket ---

    def get_cache(self, cache_id: str) -> EmbeddedAeronCache:
        return EmbeddedAeronCache(self, cache_id)

    def get_counter_cache(self, cache_id: str) -> EmbeddedCounterCache:
        return EmbeddedCounterCache(self, cache_id)

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

    async def subscribe_counter(self, cache_id, callback):
        uri = f"{self.ws_url}/api/ws/v1/counter/{cache_id}"
        while True:
            try:
                async with websockets.connect(uri) as websocket:
                    async for message in websocket:
                        data = json.loads(message)
                        event = CounterUpdateEvent.from_dict(data)
                        await callback(event)
            except (websockets.ConnectionClosed, Exception):
                await asyncio.sleep(5)
