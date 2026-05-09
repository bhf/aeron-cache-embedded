import pytest
from unittest.mock import Mock, AsyncMock
from aeron_cache.client import AeronCacheClient
from aeron_cache.embedded_cache import EmbeddedAeronCache
from aeron_cache.models import CacheUpdateEvent, PutItemResponse

@pytest.fixture
def client_mock():
    mock = Mock(spec=AeronCacheClient)
    # Using AsyncMock for async operations if needed in the future
    return mock

@pytest.fixture
def cache(client_mock):
    return EmbeddedAeronCache(client_mock, "test-cache")

def test_put_delegates_to_client(cache, client_mock):
    expected_response = PutItemResponse(cacheId="test-cache", key="key1", operationStatus="SUCCESS")
    client_mock.put_item.return_value = expected_response

    response = cache.put("key1", "val1")

    client_mock.put_item.assert_called_once_with("test-cache", "key1", "val1")
    assert response == expected_response

def test_remove_delegates_to_client(cache, client_mock):
    cache.remove("key1")
    client_mock.delete_item.assert_called_once_with("test-cache", "key1")

def test_clear_delegates_to_client(cache, client_mock):
    cache.clear()
    client_mock.delete_cache.assert_called_once_with("test-cache")

@pytest.mark.asyncio
async def test_put_async_delegates_to_client_async(cache, client_mock):
    client_mock.put_item_async = AsyncMock()
    await cache.put_async("key1", "val1")
    client_mock.put_item_async.assert_awaited_once_with("test-cache", "key1", "val1")

def test_update_local_cache_mutates_state(cache):
    # Simulate ADD_ITEM
    add_event = CacheUpdateEvent(eventType="ADD_ITEM", itemKey="my-key", itemValue="my-value", cacheId="test-cache", requestId="123")
    cache._update_local_cache(add_event)
    assert cache.get_local("my-key") == "my-value"
    
    # Simulate REMOVE_ITEM
    remove_event = CacheUpdateEvent(eventType="REMOVE_ITEM", itemKey="my-key", cacheId="test-cache", requestId="123")
    cache._update_local_cache(remove_event)
    assert cache.get_local("my-key") is None
