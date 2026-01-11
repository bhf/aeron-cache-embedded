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
