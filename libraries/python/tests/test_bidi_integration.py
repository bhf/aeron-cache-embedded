import os
import uuid
import asyncio

import pytest

from aeron_cache.bidi_client import AeronBidiClient

pytestmark = pytest.mark.skipif(
    not os.environ.get("AERON_CACHE_BASE_URL"),
    reason="Integration tests disabled: AERON_CACHE_BASE_URL not set",
)


def _ws_url():
    url = os.environ.get("AERON_CACHE_WS_URL")
    if not url:
        base = os.environ.get("AERON_CACHE_BASE_URL")
        url = base.replace("http://", "ws://").replace("https://", "wss://")
    return url


# 300 entries force the server to split the response across multiple batch frames while keeping the
# write load light enough for a cold CI RAFT cluster (populating 1000 entries timed out on CI).
_BATCH_ENTRY_COUNT = 300
# Small so each populate bulk commits well within the bidi request timeout on a slow CI cluster, and
# well under the transport's ~64KB max message length.
_BATCH_BULK_CHUNK = 20
# Value long enough that _BATCH_ENTRY_COUNT entries comfortably exceed one batch frame.
_BATCH_VALUE = "v" * 300


async def _populate_entries(client, cache_id, count, ttl=0):
    """Insert ``count`` entries (keys ``k0..k{count-1}``) via chunked bulk ADD_ITEM ops.

    Chunked into groups of ``_BATCH_BULK_CHUNK`` to keep each request frame under the
    transport's max message length.
    """
    import uuid as _uuid
    from aeron_cache.models import (
        BulkCacheOpsRequest,
        CacheOperationRequest,
        BulkOperationType,
    )

    for start in range(0, count, _BATCH_BULK_CHUNK):
        end = min(start + _BATCH_BULK_CHUNK, count)
        operations = []
        for i in range(start, end):
            kwargs = dict(
                operationType=BulkOperationType.ADD_ITEM,
                requestId=f"add-{i}",
                cacheId=cache_id,
                key=f"k{i}",
                value=_BATCH_VALUE,
            )
            if ttl > 0:
                kwargs["ttl"] = ttl
            operations.append(CacheOperationRequest(**kwargs))
        request = BulkCacheOpsRequest(requestId=str(_uuid.uuid4()), operations=operations)
        response = await client.bulk_ops(request)
        assert len(response.operationResponses) == len(operations)


@pytest.mark.asyncio
async def test_bidi_cache_lifecycle():
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-cache-{uuid.uuid4().hex[:8]}"
        assert (await client.create_cache(cache_id)).cacheId == cache_id
        assert (await client.put_item(cache_id, "k1", "v1")).operationStatus == "SUCCESS"
        assert (await client.get_item(cache_id, "k1")).value == "v1"

        await client.put_item(cache_id, "doc", '{"a":1}')
        await client.patch_item(cache_id, "doc", '{"b":2}')
        doc = (await client.get_item(cache_id, "doc")).value
        assert '"a":1' in doc and '"b":2' in doc

        items = (await client.get_cache_items(cache_id)).items
        assert {i.key for i in items} == {"k1", "doc"}

        assert (await client.delete_item(cache_id, "k1")).operationStatus == "SUCCESS"
        assert (await client.clear_cache(cache_id)).operationStatus == "SUCCESS"
        await client.delete_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_counter_lifecycle():
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-counter-{uuid.uuid4().hex[:8]}"
        await client.create_counter_cache(cache_id)
        await client.put_counter(cache_id, "hits", 10)
        assert (await client.increment_counter(cache_id, "hits", 5)).value == 15
        assert (await client.decrement_counter(cache_id, "hits", 3)).value == 12
        assert (await client.set_counter(cache_id, "hits", 100)).value == 100
        assert (await client.get_counter(cache_id, "hits")).value == 100

        await client.put_counter(cache_id, "misses", 7)
        got = await client.get_counter_items(cache_id)
        assert {i.key: i.value for i in got.items} == {"hits": 100, "misses": 7}

        await client.clear_counter_cache(cache_id)
        await client.delete_counter_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_get_stats():
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-stats-{uuid.uuid4().hex[:8]}"
        await client.create_cache(cache_id)
        await client.put_item(cache_id, "k", "v")
        stats = await client.get_stats()
        assert isinstance(stats, list)
        await client.delete_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_get_caches():
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-caches-{uuid.uuid4().hex[:8]}"
        await client.create_cache(cache_id)
        await client.put_item(cache_id, "k", "v")
        caches = await client.get_caches()
        details = next((c for c in caches if c.cacheId == cache_id), None)
        assert details is not None, f"expected {cache_id} in get_caches()"
        assert details.itemCount >= 1
        await client.delete_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_get_counter_caches():
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-counter-caches-{uuid.uuid4().hex[:8]}"
        await client.create_counter_cache(cache_id)
        await client.put_counter(cache_id, "hits", 1)
        caches = await client.get_counter_caches()
        details = next((c for c in caches if c.cacheId == cache_id), None)
        assert details is not None, f"expected {cache_id} in get_counter_caches()"
        await client.clear_counter_cache(cache_id)
        await client.delete_counter_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_get_timers():
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-timers-{uuid.uuid4().hex[:8]}"
        await client.create_cache(cache_id)
        # A timed entry schedules a pending TTL removal timer.
        await client.put_timed_item(cache_id, "ttl-key", "v", 600000)

        timers = await client.get_timers()
        assert isinstance(timers, list)
        timer = next(
            (t for t in timers if t.cacheId == cache_id and t.key == "ttl-key"),
            None,
        )
        assert timer is not None, f"expected a pending timer for {cache_id}/ttl-key"
        assert timer.timerType == "CACHE"
        assert timer.deadline > 0
        await client.delete_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_bulk_ops():
    from aeron_cache.models import (
        BulkCacheOpsRequest,
        CacheOperationRequest,
        BulkOperationType,
    )

    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-bulk-{uuid.uuid4().hex[:8]}"
        await client.create_cache(cache_id)

        req_id = str(uuid.uuid4())
        request = BulkCacheOpsRequest(
            requestId=req_id,
            operations=[
                CacheOperationRequest(
                    operationType=BulkOperationType.ADD_ITEM,
                    requestId="op-1", cacheId=cache_id, key="bk1", value="bv1",
                ),
                CacheOperationRequest(
                    operationType=BulkOperationType.ADD_ITEM,
                    requestId="op-2", cacheId=cache_id, key="bk2", value="bv2",
                ),
                CacheOperationRequest(
                    operationType=BulkOperationType.GET_ITEM,
                    requestId="op-3", cacheId=cache_id, key="bk1",
                ),
            ],
        )

        response = await client.bulk_ops(request)
        assert response.requestId == req_id
        assert len(response.operationResponses) == 3

        get_result = next(
            r for r in response.operationResponses if r.requestId == "op-3"
        )
        assert get_result.value == "bv1"

        assert (await client.get_item(cache_id, "bk2")).value == "bv2"
        await client.delete_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_cancel_item_removal():
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-cancel-{uuid.uuid4().hex[:8]}"
        await client.create_cache(cache_id)
        await client.put_timed_item(cache_id, "keep", "val", 2000)
        assert (await client.cancel_item_removal(cache_id, "keep")).key == "keep"
        await asyncio.sleep(3)
        assert (await client.get_item(cache_id, "keep")).value == "val"
        await client.delete_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_subscription():
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-sub-{uuid.uuid4().hex[:8]}"
        await client.create_cache(cache_id)

        received = asyncio.Event()
        got = []

        def on_event(ev):
            if ev.eventType == "ADD_ITEM" and ev.itemKey == "sk":
                got.append(ev)
                received.set()

        sub = await client.subscribe(cache_id, on_event)
        await client.put_item(cache_id, "sk", "sv")
        await asyncio.wait_for(received.wait(), timeout=5.0)
        assert got[0].itemValue == "sv"

        await sub.close()
        await client.delete_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_reconnect_replays_subscriptions_after_drop():
    """Force an involuntary drop (close the underlying socket, not ``close()``), then assert the next
    command transparently reconnects and a subscription created before the drop resumes delivering
    updates."""
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-recon-{uuid.uuid4().hex[:8]}"
        await client.create_cache(cache_id)

        got = set()
        client_sub = await client.subscribe(
            cache_id, lambda ev: got.add(ev.itemKey) if ev.eventType == "ADD_ITEM" else None
        )

        async def await_key(key, attempts=50):
            for _ in range(attempts):
                if key in got:
                    return True
                await asyncio.sleep(0.1)
            return False

        # Baseline: subscription delivers before any drop.
        await client.put_item(cache_id, "before", "v0")
        assert await await_key("before"), "baseline subscription update not received"

        # Force an involuntary drop by closing the underlying socket directly.
        await client._ws.close()
        for _ in range(50):
            if client._ws is None:
                break
            await asyncio.sleep(0.02)
        assert client._ws is None, "drop should have cleared the socket"

        # Next command should transparently reconnect.
        r = await client.put_item(cache_id, "after", "v1")
        assert r.operationStatus == "SUCCESS"
        assert client._ws is not None, "command after drop should have reconnected"

        # The replayed subscription must resume delivering updates. Re-registration on the new
        # connection can land slightly after reconnect (notably on a slow RAFT backend), and any event
        # produced before it is live falls in the reconnect gap and is lost. Prove the subscription
        # *eventually* resumes by producing a fresh-keyed update each round until one is delivered.
        resumed = False
        for i in range(40):
            key = f"postreconnect-{i}"
            await client.put_item(cache_id, key, "v2")
            if await await_key(key, attempts=5):
                resumed = True
                break
        assert resumed, "subscription did not resume after reconnect (replay failed)"

        await client_sub.close()
        await client.delete_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_subscription_key_filter():
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-sub-keys-{uuid.uuid4().hex[:8]}"
        await client.create_cache(cache_id)

        received = []

        def on_event(ev):
            if ev.eventType == "ADD_ITEM" and ev.itemKey:
                received.append(ev.itemKey)

        # Subscribe filtered to only "key1".
        sub = await client.subscribe(cache_id, on_event, key="key1")
        await client.put_item(cache_id, "key1", "v1")
        await client.put_item(cache_id, "key2", "v2")
        await asyncio.sleep(2.0)

        await sub.close()
        assert "key1" in received, f"Expected key1 event, got {received}"
        assert "key2" not in received, f"key2 should be filtered out, got {received}"
        await client.delete_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_subscription_patch_mode():
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-sub-patch-{uuid.uuid4().hex[:8]}"
        await client.create_cache(cache_id)
        await client.put_item(cache_id, "doc", '{"a":1}')

        patch_event = asyncio.Event()

        def on_event(ev):
            if ev.eventType == "PATCH_ITEM" and ev.itemKey == "doc":
                patch_event.set()

        sub = await client.subscribe(cache_id, on_event, mode="patch")
        await client.patch_item(cache_id, "doc", '{"b":2}')
        try:
            await asyncio.wait_for(patch_event.wait(), timeout=5.0)
        finally:
            await sub.close()
        await client.delete_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_counter_subscription():
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-counter-sub-{uuid.uuid4().hex[:8]}"
        await client.create_counter_cache(cache_id)

        received = asyncio.Event()
        got = []

        def on_event(ev):
            if ev.eventType == "ADD_ITEM" and ev.itemKey == "ws-counter":
                got.append(ev)
                received.set()

        sub = await client.subscribe_counter(cache_id, on_event)
        await client.put_counter(cache_id, "ws-counter", 7)
        await asyncio.wait_for(received.wait(), timeout=5.0)
        assert got[0].itemValue == 7

        await sub.close()
        await client.delete_counter_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_get_cache_items_multi_batch_reassembly():
    # The server streams GET_CACHE_ENTRIES back as one or more "entries" frames terminated by an
    # endOfBatch marker; AeronBidiClient accumulates them internally and only resolves once the
    # marker arrives (see _dispatch). endOfBatch is therefore not observable from Python, so this
    # is a reassembly-COMPLETENESS test: populating _BATCH_ENTRY_COUNT 300-char entries forces the
    # server to split the response across multiple batches, and getting ALL entries back with the
    # correct values proves the client reassembled every batch.
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-batch-entries-{uuid.uuid4().hex[:8]}"
        await client.create_cache(cache_id)
        await _populate_entries(client, cache_id, _BATCH_ENTRY_COUNT)

        items = (await client.get_cache_items(cache_id)).items
        by_key = {i.key: i.value for i in items}
        assert len(by_key) == _BATCH_ENTRY_COUNT, (
            "every entry should be reassembled from the batched response"
        )
        for i in range(_BATCH_ENTRY_COUNT):
            assert by_key.get(f"k{i}") == _BATCH_VALUE, f"missing/incorrect value for k{i}"

        await client.delete_cache(cache_id)


@pytest.mark.asyncio
async def test_bidi_get_timers_multi_batch_reassembly():
    # Each timed entry schedules a pending TTL removal timer; a large TTL keeps them all pending.
    # get_timers is global and its "timers" frames are likewise accumulated until endOfBatch, so
    # this is again a reassembly-COMPLETENESS test: _BATCH_ENTRY_COUNT pending timers force a
    # server-side split, and recovering all of our cache's timers proves full reassembly.
    async with AeronBidiClient(_ws_url()) as client:
        cache_id = f"bidi-batch-timers-{uuid.uuid4().hex[:8]}"
        await client.create_cache(cache_id)
        await _populate_entries(client, cache_id, _BATCH_ENTRY_COUNT, ttl=600000)

        timers = await client.get_timers()
        ours = [t for t in timers if t.cacheId == cache_id]
        assert len(ours) == _BATCH_ENTRY_COUNT, (
            f"every pending timer for {cache_id} should be reassembled"
        )
        keys = set()
        for timer in ours:
            assert timer.timerType == "CACHE"
            assert timer.deadline > 0
            keys.add(timer.key)
        assert keys == {f"k{i}" for i in range(_BATCH_ENTRY_COUNT)}

        await client.delete_cache(cache_id)
