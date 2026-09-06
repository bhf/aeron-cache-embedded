import asyncio
import sys

from aeron_cache.client import AeronCacheClient
from aeron_cache.embedded_cache import EmbeddedAeronCache
from aeron_cache.embedded_counter_cache import EmbeddedCounterCache


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
        print(f"Created cache: {response.cacheId}")
    except:
        pass

    cache = EmbeddedAeronCache(client, "async-sample-cache")

    print("Putting key 'async-key' -> 'async-value' asynchronously")
    put_response = await cache.put_async("async-key", "async-value")
    print(f"Put operation status: {put_response.status}")

    await asyncio.sleep(0.1)

    get_response = await cache.get_async("async-key")
    print(f"Read key 'async-key': {get_response.value or 'not found'}")

    # --- Counter operations ---
    try:
        counter_resp = await client.create_counter_cache_async("async-counter-cache")
        print(f"Created counter cache: {counter_resp.cacheId}")
    except Exception:
        pass

    counters = EmbeddedCounterCache(client, "async-counter-cache")

    print("Putting counter 'requests' -> 10 asynchronously")
    await counters.put_async("requests", 10)
    inc = await counters.increment_async("requests", 5)
    print(f"Incremented 'requests' by 5 -> {inc.value}")
    dec = await counters.decrement_async("requests", 3)
    print(f"Decremented 'requests' by 3 -> {dec.value}")
    set_resp = await counters.set_async("requests", 100)
    print(f"Set 'requests' -> {set_resp.value}")
    get_counter = await counters.get_async("requests")
    print(f"Read counter 'requests': {get_counter.value}")

if __name__ == "__main__":
    asyncio.run(main())
