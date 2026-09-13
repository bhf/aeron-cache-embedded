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
