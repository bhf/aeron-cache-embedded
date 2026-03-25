import asyncio
import sys

from aeron_cache.client import AeronCacheClient
from aeron_cache.embedded_cache import EmbeddedAeronCache


async def main():
    base_url = "http://localhost:7070"
    ws_url = "http://localhost:7071"
    if len(sys.argv) > 1:
        base_url = sys.argv[1]
        ws_url = sys.argv[2]

    print(f"Starting Async Sample against {base_url}, ws={ws_url}")

    client = AeronCacheClient(base_url, ws_url)

    try:
        response = await client.create_cache_async("async-sample-cache")
        print(f"Created cache: {response.cache_id}")
    except:
        pass

    cache = EmbeddedAeronCache(client, "async-sample-cache")

    print("Putting key 'async-key' -> 'async-value' asynchronously")
    put_response = await cache.put_async("async-key", "async-value")
    print(f"Put operation status: {put_response.status}")

    await asyncio.sleep(0.1)

    get_response = await cache.get_async("async-key")
    print(f"Read key 'async-key': {get_response.value or 'not found'}")

if __name__ == "__main__":
    asyncio.run(main())
