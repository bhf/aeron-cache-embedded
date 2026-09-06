import asyncio
import sys
from aeron_cache.client import AeronCacheClient
from aeron_cache.embedded_cache import EmbeddedAeronCache
from aeron_cache.embedded_counter_cache import EmbeddedCounterCache

async def main():
    base_url = "http://localhost:7070"
    ws_url = "ws://localhost:7071"
    if len(sys.argv) > 1:
        base_url = sys.argv[1]
        ws_url = sys.argv[2]

    print(f"Starting Streaming Sample against {base_url}, ws={ws_url}")

    client = AeronCacheClient(base_url, ws_url)

    # Ensure cache exists
    try:
        await client.create_cache_async("streaming-sample-cache")
    except:
        pass

    cache = EmbeddedAeronCache(client, "streaming-sample-cache")
    print("Connected. Waiting for updates on 'streaming-sample-cache'...")

    async def on_changes(data):
        print(f"[Python] Observed change: {data}")

    async def poller():
        last_val = None
        while True:
            val = cache.get_local("streaming-key")
            if val and val != last_val:
                print(f"[Python-Poller] 'streaming-key' updated: {val}")
                last_val = val
            await asyncio.sleep(1)

    asyncio.create_task(poller())

    # --- Counter streaming (with hydration) ---
    try:
        await client.create_counter_cache_async("streaming-counter-cache")
    except Exception:
        pass

    counters = EmbeddedCounterCache(client, "streaming-counter-cache")

    async def on_counter_change(event):
        print(f"[Python] Counter update: {event.eventType} {event.itemKey} -> {event.itemValue}")

    async def counter_ticker():
        while True:
            try:
                await counters.increment_async("tick", 1)
            except Exception:
                pass
            await asyncio.sleep(2)

    asyncio.create_task(counters.subscribe(on_counter_change, hydrate=True))
    asyncio.create_task(counter_ticker())

    # This will block forever receiving updates, with initial hydration
    print("Subscribing with hydration...")
    await cache.subscribe(on_changes, hydrate=True)

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass
