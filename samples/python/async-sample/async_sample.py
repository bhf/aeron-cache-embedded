import asyncio
import sys
from aeron_cache.embedded_cache import EmbeddedAeronCache

async def main():
    base_url = "http://localhost:8080"
    if len(sys.argv) > 1:
        base_url = sys.argv[1]

    print(f"Starting Async Sample against {base_url}")

    async with EmbeddedAeronCache(base_url) as cache:
        print("Putting key 'async-key' -> 'async-value' asynchronously")
        
        # Fire off the put
        task = asyncio.create_task(cache.put("async-key", "async-value"))
        
        # Do other work if needed...
        
        # Wait for completion
        await task
        print("Put operation completed.")

        # Allow time to propagate
        await asyncio.sleep(0.1)

        val = await cache.get("async-key")
        print(f"Read key 'async-key': {val or 'not found'}")
            
if __name__ == "__main__":
    asyncio.run(main())
