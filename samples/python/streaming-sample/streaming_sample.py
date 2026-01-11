import asyncio
import sys
from aeron_cache.embedded_cache import EmbeddedAeronCache

async def main():
    base_url = "http://localhost:8080"
    if len(sys.argv) > 1:
        base_url = sys.argv[1]

    print(f"Starting Streaming Sample against {base_url}")

    async with EmbeddedAeronCache(base_url) as cache:
        print("Connected. Waiting for updates on 'shared-key'...")
        
        last_value = None
        while True:
            current = await cache.get("shared-key")
            if current != last_value:
                print(f"Observed change in 'shared-key': {current}")
                last_value = current
            
            await asyncio.sleep(1)

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass
