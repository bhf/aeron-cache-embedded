import asyncio
import sys
from aeron_cache.client import AeronCacheClient
from aeron_cache.embedded_cache import EmbeddedAeronCache

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
        print(f"Observed change: {data}")

    async def poller():
        while True:
            val = cache.get_local("streaming-key")
            print(f"Polled 'streaming-key': {val}")
            await asyncio.sleep(1)

    asyncio.create_task(poller())

    # This will block forever receiving updates
    await cache.subscribe(on_changes)

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass
