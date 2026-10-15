"""Transport-neutral facade over the client libraries, so a single workload can drive the HTTP or
bidi-WebSocket client. Both expose the same cache + counter operations and return the same model types,
so each implementation is a thin wrapper.

The HTTP client is synchronous; the bidi client is asyncio-based, so ``BidiSoakClient`` owns a private
event loop and drives each async op to completion — presenting the same synchronous facade to the
single-writer workload.

Methods return the operation's ``operationStatus`` string where the workload asserts on it, the
read-back value for reads, or ``None`` for an absent key.
"""
import asyncio
from typing import Optional

from aeron_cache.client import AeronCacheClient
from aeron_cache.bidi_client import AeronBidiClient

SUCCESS = "SUCCESS"


class SoakClient:
    def name(self) -> str:
        raise NotImplementedError

    def create_kv_cache(self, cache_id: str) -> None:
        raise NotImplementedError

    def create_counter_cache(self, cache_id: str) -> None:
        raise NotImplementedError

    def put(self, cache_id: str, key: str, value: str) -> str:
        raise NotImplementedError

    def get(self, cache_id: str, key: str) -> Optional[str]:
        raise NotImplementedError

    def remove(self, cache_id: str, key: str) -> str:
        raise NotImplementedError

    def clear(self, cache_id: str) -> str:
        raise NotImplementedError

    def delete_cache(self, cache_id: str) -> str:
        raise NotImplementedError

    def list_kv(self, cache_id: str) -> dict[str, str]:
        raise NotImplementedError

    def put_counter(self, cache_id: str, key: str, value: int) -> str:
        raise NotImplementedError

    def increment_counter(self, cache_id: str, key: str, amount: int) -> int:
        raise NotImplementedError

    def decrement_counter(self, cache_id: str, key: str, amount: int) -> int:
        raise NotImplementedError

    def set_counter(self, cache_id: str, key: str, value: int) -> int:
        raise NotImplementedError

    def get_counter(self, cache_id: str, key: str) -> Optional[int]:
        raise NotImplementedError

    def close(self) -> None:
        raise NotImplementedError


class HttpSoakClient(SoakClient):
    """SoakClient over the HTTP/REST AeronCacheClient."""

    def __init__(self, base_url: str, ws_url: str) -> None:
        self._client = AeronCacheClient(base_url, ws_url)

    def name(self) -> str:
        return "http"

    def create_kv_cache(self, cache_id: str) -> None:
        self._client.create_cache(cache_id)

    def create_counter_cache(self, cache_id: str) -> None:
        self._client.create_counter_cache(cache_id)

    def put(self, cache_id: str, key: str, value: str) -> str:
        return self._client.put_item(cache_id, key, value).operationStatus

    def get(self, cache_id: str, key: str) -> Optional[str]:
        r = self._client.get_item(cache_id, key)
        return r.value if r.operationStatus == SUCCESS else None

    def remove(self, cache_id: str, key: str) -> str:
        return self._client.delete_item(cache_id, key).operationStatus

    def clear(self, cache_id: str) -> str:
        return self._client.clear_cache(cache_id).operationStatus

    def delete_cache(self, cache_id: str) -> str:
        return self._client.delete_cache(cache_id).operationStatus

    def list_kv(self, cache_id: str) -> dict[str, str]:
        return {item.key: item.value for item in self._client.get_cache_items(cache_id).items}

    def put_counter(self, cache_id: str, key: str, value: int) -> str:
        return self._client.put_counter(cache_id, key, value).operationStatus

    def increment_counter(self, cache_id: str, key: str, amount: int) -> int:
        return self._client.increment_counter(cache_id, key, amount).value

    def decrement_counter(self, cache_id: str, key: str, amount: int) -> int:
        return self._client.decrement_counter(cache_id, key, amount).value

    def set_counter(self, cache_id: str, key: str, value: int) -> int:
        return self._client.set_counter(cache_id, key, value).value

    def get_counter(self, cache_id: str, key: str) -> Optional[int]:
        r = self._client.get_counter(cache_id, key)
        return r.value if r.operationStatus == SUCCESS else None

    def close(self) -> None:
        # AeronCacheClient holds no long-lived connection to close.
        pass


class BidiSoakClient(SoakClient):
    """SoakClient over the asyncio bidirectional-WebSocket AeronBidiClient.

    Owns a private event loop so the synchronous single-writer workload can drive the async client.
    """

    def __init__(self, ws_url: str) -> None:
        self._loop = asyncio.new_event_loop()
        self._client = AeronBidiClient(ws_url)
        self._run(self._client.connect())

    def _run(self, coro):
        return self._loop.run_until_complete(coro)

    def name(self) -> str:
        return "bidi"

    def create_kv_cache(self, cache_id: str) -> None:
        self._run(self._client.create_cache(cache_id))

    def create_counter_cache(self, cache_id: str) -> None:
        self._run(self._client.create_counter_cache(cache_id))

    def put(self, cache_id: str, key: str, value: str) -> str:
        return self._run(self._client.put_item(cache_id, key, value)).operationStatus

    def get(self, cache_id: str, key: str) -> Optional[str]:
        r = self._run(self._client.get_item(cache_id, key))
        return r.value if r.operationStatus == SUCCESS else None

    def remove(self, cache_id: str, key: str) -> str:
        return self._run(self._client.delete_item(cache_id, key)).operationStatus

    def clear(self, cache_id: str) -> str:
        return self._run(self._client.clear_cache(cache_id)).operationStatus

    def delete_cache(self, cache_id: str) -> str:
        return self._run(self._client.delete_cache(cache_id)).operationStatus

    def list_kv(self, cache_id: str) -> dict[str, str]:
        resp = self._run(self._client.get_cache_items(cache_id))
        return {item.key: item.value for item in resp.items}

    def put_counter(self, cache_id: str, key: str, value: int) -> str:
        return self._run(self._client.put_counter(cache_id, key, value)).operationStatus

    def increment_counter(self, cache_id: str, key: str, amount: int) -> int:
        return self._run(self._client.increment_counter(cache_id, key, amount)).value

    def decrement_counter(self, cache_id: str, key: str, amount: int) -> int:
        return self._run(self._client.decrement_counter(cache_id, key, amount)).value

    def set_counter(self, cache_id: str, key: str, value: int) -> int:
        return self._run(self._client.set_counter(cache_id, key, value)).value

    def get_counter(self, cache_id: str, key: str) -> Optional[int]:
        r = self._run(self._client.get_counter(cache_id, key))
        return r.value if r.operationStatus == SUCCESS else None

    def close(self) -> None:
        try:
            self._run(self._client.close())
        finally:
            self._loop.close()
