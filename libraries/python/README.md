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
