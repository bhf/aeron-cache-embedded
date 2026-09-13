# Aeron Cache Python Client

This is the Python client library for Aeron Cache.

## Installation

```bash
pip install aeron-cache-embedded-client
```

## Usage

```python
from aeron_cache.client import AeronCacheClient
import asyncio

async def main():
    client = AeronCacheClient("http://localhost:7070", "ws://localhost:7071")
    cache = client.get_cache("my-cache")
    
    await cache.put_async("key", "value")
    print(await cache.get_async("key"))

if __name__ == "__main__":
    asyncio.run(main())
```

## Counters

Counter caches hold `int64` values and add `increment`, `decrement`, `set`, and timed-put operations:

```python
from aeron_cache.client import AeronCacheClient

client = AeronCacheClient("http://localhost:7070", "ws://localhost:7071")
client.create_counter_cache("counter-cache")
counters = client.get_counter_cache("counter-cache")

counters.put("requests", 10)
counters.increment("requests", 5)  # -> 15
counters.decrement("requests", 3)  # -> 12
counters.set("requests", 100)      # -> 100
print(counters.get("requests").value)
```

Counter operations are also available via `bulk_ops` using the counter `BulkOperationType` values and the `counterValue` field on `CacheOperationRequest`.

## Inspection & management operations

Beyond basic CRUD, the client exposes the full HTTP management surface (each has an `_async` variant):

```python
client.patch_item("my-cache", "doc", '{"b":2}')   # deep-merge into an existing item
client.cancel_item_removal("my-cache", "key")      # cancel a scheduled TTL removal
client.get_caches()                                # -> list[CacheDetails] (cacheId, itemCount)
client.get_stats()                                 # -> CacheStatsResponse (totals + errorCount)

# Counter equivalents:
client.get_counter_items("counter-cache")          # -> GetCountersResponse (items: list[CounterItem])
client.clear_counter_cache("counter-cache")
client.cancel_counter_item_removal("counter-cache", "key")
client.get_counter_caches()                         # -> list[CacheDetails]
client.get_counter_stats()                          # -> CacheStatsResponse
```

## WebSocket subscriptions

`subscribe` accepts two optional filters from the WebSocket API:

- `keys` — a comma-separated string (or list) of `cacheId:key` / bare `key` tokens that restrict the subscription to specific keys.
- `mode` — `"full"` (default, full values as `ADD_ITEM` events) or `"patch"` (cache-only; streams only the changed fields of patched items as `PATCH_ITEM` events).

```python
# Only receive updates for "key1", as patch deltas:
await client.subscribe("my-cache", on_event, keys="key1", mode="patch")

# Counter subscriptions support keys (patch mode is cache-only):
await client.subscribe_counter("counter-cache", on_event, keys=["counter-cache:hits"])
```
