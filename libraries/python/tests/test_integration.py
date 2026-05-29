import os
import pytest
import uuid
import asyncio
from aeron_cache.client import AeronCacheClient

# Skip the whole module if not provided
pytestmark = pytest.mark.skipif(
    not os.environ.get("AERON_CACHE_BASE_URL"),
    reason="Integration tests disabled: AERON_CACHE_BASE_URL not set"
)

@pytest.fixture
def base_url():
    return os.environ.get("AERON_CACHE_BASE_URL")

@pytest.fixture
def ws_url(base_url):
    url = os.environ.get("AERON_CACHE_WS_URL")
    if not url and base_url:
        url = base_url.replace("http://", "ws://").replace("https://", "wss://")
    return url

@pytest.fixture
def client(base_url, ws_url):
    return AeronCacheClient(base_url, ws_url)

def test_cache_operations(client):
    cache_id = f"it-cache-{uuid.uuid4().hex[:8]}"

    # Create Cache
    create_resp = client.create_cache(cache_id)
    assert create_resp is not None
    assert create_resp.cacheId == cache_id

    embedded = client.get_cache(cache_id)

    # Put an item
    put_resp = embedded.put("key1", "val1")
    assert put_resp is not None
    assert put_resp.key == "key1"

    # Get the item
    get_resp = embedded.get("key1")
    assert get_resp is not None
    assert get_resp.value == "val1"

    # Remove the item
    del_resp = embedded.remove("key1")
    assert del_resp is not None

    # Get the item again, should handle 404 cleanly
    get_resp2 = embedded.get("key1")
    assert get_resp2 is not None
    assert get_resp2.operationStatus == "UNKNOWN_KEY" or get_resp2.value is None

@pytest.mark.asyncio
async def test_websocket_subscription(client):
    cache_id = f"it-ws-{uuid.uuid4().hex[:8]}"
    client.create_cache(cache_id)
    embedded = client.get_cache(cache_id)

    event_received = asyncio.Event()

    async def on_event(event):
        if event.eventType == "ADD_ITEM" and event.itemKey == "ws-key":
            event_received.set()

    # Subscribe is an infinite blocking loop, so run it as a task
    sub_task = asyncio.create_task(embedded.subscribe(on_event))

    # Give websocket time to connect
    await asyncio.sleep(1.0)

    # Trigger change via REST
    embedded.put("ws-key", "ws-val")

    # Wait up to 5 seconds for the event
    try:
        await asyncio.wait_for(event_received.wait(), timeout=5.0)
    finally:
        sub_task.cancel()

    assert embedded.get_local("ws-key") == "ws-val"
    embedded.clear()

def test_get_and_clear_cache(client):
    cache_id = f"it-cache2-{uuid.uuid4().hex[:8]}"

    # Create Cache
    client.create_cache(cache_id)

    client.put_item(cache_id, "key1", "val1")
    client.put_item(cache_id, "key2", "val2")

    get_resp = client.get_cache_items(cache_id)
    assert len(get_resp.items) == 2

    clear_resp = client.clear_cache(cache_id)
    assert clear_resp.operationStatus == "SUCCESS"

def test_bulk_operations(client):
    from aeron_cache.models import BulkCacheOpsRequest, CacheOperationRequest, BulkOperationType
    import uuid

    cache_id = f"it-bulk-{uuid.uuid4().hex[:8]}"
    req_id = str(uuid.uuid4())

    request = BulkCacheOpsRequest(
        requestId=req_id,
        operations=[
            CacheOperationRequest(
                operationType=BulkOperationType.CREATE_CACHE,
                requestId="op-1",
                cacheId=cache_id
            ),
            CacheOperationRequest(
                operationType=BulkOperationType.ADD_ITEM,
                requestId="op-2",
                cacheId=cache_id,
                key="bulk-key",
                value="bulk-val"
            ),
            CacheOperationRequest(
                operationType=BulkOperationType.GET_ITEM,
                requestId="op-3",
                cacheId=cache_id,
                key="bulk-key"
            )
        ]
    )

    response = client.bulk_ops(request)
    assert response.requestId == req_id
    assert len(response.operationResponses) == 3
    
    # Verify CREATE_CACHE
    assert response.operationResponses[0].requestId == "op-1"
    
    # Verify ADD_ITEM
    assert response.operationResponses[1].requestId == "op-2"
    
    # Verify GET_ITEM
    assert response.operationResponses[2].requestId == "op-3"
    assert response.operationResponses[2].value == "bulk-val"

    get_resp2 = client.get_cache_items(cache_id)
    assert len(get_resp2.items) == 1
    assert get_resp2.items[0].key == "bulk-key"
