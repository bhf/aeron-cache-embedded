import pytest
from unittest.mock import Mock, AsyncMock
from aeron_cache.client import AeronCacheClient
from aeron_cache.embedded_counter_cache import EmbeddedCounterCache
from aeron_cache.models import CounterUpdateEvent, CounterResponse

@pytest.fixture
def client_mock():
    return Mock(spec=AeronCacheClient)

@pytest.fixture
def cache(client_mock):
    return EmbeddedCounterCache(client_mock, "test-counter")

def test_put_delegates_to_client(cache, client_mock):
    client_mock.put_counter.return_value = CounterResponse(cacheId="test-counter", key="hits", value=42, operationStatus="SUCCESS")
    cache.put("hits", 42)
    client_mock.put_counter.assert_called_once_with("test-counter", "hits", 42)

def test_increment_delegates_to_client(cache, client_mock):
    cache.increment("hits", 1)
    client_mock.increment_counter.assert_called_once_with("test-counter", "hits", 1)

def test_decrement_delegates_to_client(cache, client_mock):
    cache.decrement("hits", 1)
    client_mock.decrement_counter.assert_called_once_with("test-counter", "hits", 1)

def test_set_delegates_to_client(cache, client_mock):
    cache.set("hits", 100)
    client_mock.set_counter.assert_called_once_with("test-counter", "hits", 100)

def test_put_timed_delegates_to_client(cache, client_mock):
    cache.put_timed("hits", 5, 1000)
    client_mock.put_timed_counter.assert_called_once_with("test-counter", "hits", 5, 1000)

def test_clear_delegates_to_client(cache, client_mock):
    cache.clear()
    client_mock.delete_counter_cache.assert_called_once_with("test-counter")

@pytest.mark.asyncio
async def test_increment_async_delegates(cache, client_mock):
    client_mock.increment_counter_async = AsyncMock()
    await cache.increment_async("hits", 2)
    client_mock.increment_counter_async.assert_awaited_once_with("test-counter", "hits", 2)

def test_update_local_cache_mutates_state(cache):
    cache._update_local_cache(CounterUpdateEvent(eventType="ADD_ITEM", itemKey="hits", itemValue=42, cacheId="test-counter", requestId="1"))
    assert cache.get_local("hits") == 42

    # Zero is a valid counter value and must be stored, not treated as missing.
    cache._update_local_cache(CounterUpdateEvent(eventType="ADD_ITEM", itemKey="hits", itemValue=0, cacheId="test-counter", requestId="1"))
    assert cache.get_local("hits") == 0

    cache._update_local_cache(CounterUpdateEvent(eventType="REMOVE_ITEM", itemKey="hits", cacheId="test-counter", requestId="1"))
    assert cache.get_local("hits") is None
