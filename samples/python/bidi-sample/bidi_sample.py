import asyncio
import sys

import websockets

from aeron_cache.bidi_client import AeronBidiClient, BidiError
from aeron_cache.models import (
    BulkCacheOpsRequest,
    BulkOperationType,
    CacheOperationRequest,
)


async def main():
    ws_url = "ws://localhost:7071"
    if len(sys.argv) > 1:
        ws_url = sys.argv[1]

    print(f"Starting Bidi (bidirectional WebSocket) Sample against ws={ws_url}")

    cache_id = "bidi-sample-cache"
    counter_cache_id = "bidi-sample-counter-cache"

    try:
        async with AeronBidiClient(ws_url) as client:
            # --- Cache lifecycle ---
            create = await client.create_cache(cache_id)
            print(f"Created cache: {create.cacheId} (status={create.operationStatus})")

            # --- put + get ---
            print("Putting key 'greeting' -> 'hello'")
            put = await client.put_item(cache_id, "greeting", "hello")
            print(f"Put operation status: {put.operationStatus}")

            got = await client.get_item(cache_id, "greeting")
            print(f"Read key 'greeting': {got.value or 'not found'}")

            # --- patch (merge) ---
            print("Putting key 'doc' -> {\"a\":1}")
            await client.put_item(cache_id, "doc", '{"a":1}')
            print("Patching key 'doc' with {\"b\":2}")
            await client.patch_item(cache_id, "doc", '{"b":2}')
            merged = await client.get_item(cache_id, "doc")
            print(f"Merged 'doc' value: {merged.value}")

            # --- counter demo ---
            counter_create = await client.create_counter_cache(counter_cache_id)
            print(f"Created counter cache: {counter_create.cacheId} (status={counter_create.operationStatus})")

            print("Putting counter 'requests' -> 10")
            await client.put_counter(counter_cache_id, "requests", 10)
            inc = await client.increment_counter(counter_cache_id, "requests", 5)
            print(f"Incremented 'requests' by 5 -> {inc.value}")
            counter = await client.get_counter(counter_cache_id, "requests")
            print(f"Read counter 'requests': {counter.value}")

            # --- bulk read + stats ---
            items = await client.get_cache_items(cache_id)
            print(f"Cache items ({len(items.items)}):")
            for item in items.items:
                print(f"  {item.key} = {item.value}")

            stats = await client.get_stats()
            print(f"Cache stats ({len(stats)} entries):")
            for stat in stats:
                print(f"  {stat.cacheId}: size={stat.size} added={stat.addedCount} removed={stat.removedCount}")

            # --- bulk operations ---
            # A single bulk frame carries a batch of operations; the server streams back
            # per-operation results, each echoing its own requestId.
            bulk_request = BulkCacheOpsRequest(
                requestId="bidi-bulk-1",
                operations=[
                    CacheOperationRequest(
                        operationType=BulkOperationType.ADD_ITEM,
                        requestId="op-1", cacheId=cache_id, key="bk1", value="bv1"
                    ),
                    CacheOperationRequest(
                        operationType=BulkOperationType.ADD_ITEM,
                        requestId="op-2", cacheId=cache_id, key="bk2", value="bv2"
                    ),
                    CacheOperationRequest(
                        operationType=BulkOperationType.GET_ITEM,
                        requestId="op-3", cacheId=cache_id, key="bk1"
                    ),
                ],
            )
            bulk_response = await client.bulk_ops(bulk_request)
            print("Bulk operation responses:")
            for op_resp in bulk_response.operationResponses:
                suffix = f" ({op_resp.value})" if op_resp.value else ""
                print(f"  {op_resp.requestId} -> {op_resp.status}{suffix}")

            # --- timers ---
            # A timed entry schedules a pending TTL removal timer; get_timers lists all pending
            # timers (cache + counter), each tagged with its type.
            await client.put_timed_item(cache_id, "expiring", "gone-soon", 600000)
            print("Listing all pending TTL timers:")
            for timer in await client.get_timers():
                print(f"  [{timer.timerType}] {timer.cacheId}/{timer.key} fires at {timer.deadline}")

            # --- live subscription ---
            received = asyncio.Event()
            last_event = {}

            def on_event(event):
                last_event["event"] = event
                received.set()

            print("Subscribing to live updates on 'bidi-sample-cache'")
            sub = await client.subscribe(cache_id, on_event)

            print("Putting key 'live' -> 'streamed' to trigger an event")
            await client.put_item(cache_id, "live", "streamed")

            try:
                await asyncio.wait_for(received.wait(), timeout=5.0)
                event = last_event.get("event")
                print(
                    f"Received live event: type={event.eventType} "
                    f"key={event.itemKey} value={event.itemValue}"
                )
            except asyncio.TimeoutError:
                print("No live event received within 5s")

            await sub.close()
            print("Closed subscription")

            # --- teardown ---
            deleted = await client.delete_cache(cache_id)
            print(f"Deleted cache: {deleted.cacheId} (status={deleted.operationStatus})")
            deleted_counters = await client.delete_counter_cache(counter_cache_id)
            print(f"Deleted counter cache: {deleted_counters.cacheId} (status={deleted_counters.operationStatus})")

    except (OSError, ConnectionError, websockets.exceptions.WebSocketException) as e:
        print(f"Could not connect to the bidi WebSocket at {ws_url}: {e}")
        print("Is the backend running (HTTP 7070, WS 7071)?")
        sys.exit(1)
    except BidiError as e:
        print(f"Server returned an error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
