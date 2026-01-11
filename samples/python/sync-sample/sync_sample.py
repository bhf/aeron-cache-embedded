import asyncio
import sys
from aeron_cache.embedded_cache import EmbeddedAeronCache

async def main():
    base_url = "http://localhost:8080"
    if len(sys.argv) > 1:
        base_url = sys.argv[1]

    print(f"Starting Sync Sample against {base_url}")

    # Note: The embedded cache is inherently async under the hood (asyncio), 
    # but we can simulate a sync-like flow in this async main helper.
    async with EmbeddedAeronCache(base_url) as cache:
        print("Putting key 'sync-key' -> 'sync-value'")
        await cache.put("sync-key", "sync-value")

        # Allow time to propagate
        await asyncio.sleep(0.1)

        val = await cache.get("sync-key")
        if val:
            print(f"Read back key 'sync-key': {val}")
        else:
            print("Key 'sync-key' not found in local cache yet.")
            
if __name__ == "__main__":
    asyncio.run(main())
