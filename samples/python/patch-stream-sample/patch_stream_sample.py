import asyncio
import sys

from aeron_cache.client import AeronCacheClient


async def main():
    """Demonstrates server-side synthetic PATCH streaming.

    Subscribes in ``patch`` mode and then writes evolving *full* JSON values with ordinary ``put_item``
    calls (never ``patch_item``). As long as a patch-mode subscriber is listening, the cache service turns
    those puts into patch traffic: the first put of a key arrives as a full ``ADD_ITEM`` baseline, each later
    overwrite as a ``PATCH_ITEM`` delta of only the changed fields, an identical (no-op) put is suppressed,
    and a non-JSON value falls back to a full ``ADD_ITEM``. The EmbeddedObjectCache deep-merges each delta
    (RFC 7386), so ``get`` always holds the full, reconstructed object.
    """
    base_url = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:7070"
    ws_url = sys.argv[2] if len(sys.argv) > 2 else "ws://localhost:7071"

    print(f"Starting Patch Stream Sample against {base_url}, ws={ws_url}")

    client = AeronCacheClient(base_url, ws_url)
    cache_id = "patch-stream-sample-cache"
    try:
        await client.create_cache_async(cache_id)
    except Exception:
        pass  # Probably already exists.

    # Start from a clean slate so the first put is a fresh insert (an ADD_ITEM baseline) rather than an
    # overwrite of a value left over from a previous run (which would arrive as a PATCH_ITEM delta).
    try:
        await client.delete_item_async(cache_id, "doc")
    except Exception:
        pass

    oc = client.get_object_cache(cache_id)

    def on_event(event):
        if event.itemKey != "doc":
            return
        print(f"[Python] <- {event.eventType} delta={event.itemValue}  reconstructed={oc.get('doc')}")

    # Subscribe in patch mode BEFORE the key exists. The object cache subscribe wraps the client reader
    # loop, so run it as a task; the first put arrives as an ADD_ITEM baseline and every later put as a
    # PATCH_ITEM delta the mirror deep-merges locally.
    asyncio.create_task(oc.subscribe(on_event, mode="patch"))
    await asyncio.sleep(0.8)  # let the subscription register before the first put

    states = [
        '{"user":{"name":"ada","age":36},"status":"active"}',
        '{"user":{"name":"ada","age":37},"status":"active"}',  # delta: user.age -> 37
        '{"user":{"name":"ada","age":37},"status":"away"}',    # delta: status -> away
        '{"user":{"name":"ada","age":37},"status":"away"}',    # identical -> suppressed
        '{"user":{"name":"ada","age":37}}',                      # delta: status -> null (removed)
    ]

    i = 0
    while True:
        state = states[i % len(states)]
        print(f"[Python] -> put doc = {state}")
        await client.put_item_async(cache_id, "doc", state)
        i += 1
        await asyncio.sleep(2.5)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass
