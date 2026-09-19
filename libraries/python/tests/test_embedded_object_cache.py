import pytest
from unittest.mock import Mock, AsyncMock
from aeron_cache.client import AeronCacheClient
from aeron_cache.embedded_object_cache import EmbeddedObjectCache, _deep_merge
from aeron_cache.models import CacheUpdateEvent, PutItemResponse, PatchItemResponse


@pytest.fixture
def client_mock():
    return Mock(spec=AeronCacheClient)


@pytest.fixture
def cache(client_mock):
    return EmbeddedObjectCache(client_mock, "test-cache")


def _event(event_type, key=None, value=None):
    return CacheUpdateEvent(eventType=event_type, itemKey=key, itemValue=value,
                            cacheId="test-cache", requestId="123")


def test_put_serializes_object_and_delegates(cache, client_mock):
    client_mock.put_item.return_value = PutItemResponse(cacheId="test-cache", key="doc", operationStatus="SUCCESS")
    cache.put("doc", {"a": 1})
    client_mock.put_item.assert_called_once_with("test-cache", "doc", '{"a": 1}')


def test_patch_serializes_fragment_and_delegates(cache, client_mock):
    client_mock.patch_item.return_value = PatchItemResponse(cacheId="test-cache", key="doc", operationStatus="SUCCESS")
    cache.patch("doc", {"b": 2})
    client_mock.patch_item.assert_called_once_with("test-cache", "doc", '{"b": 2}')


def test_add_item_stores_parsed_object(cache):
    cache._update_local_cache(_event("ADD_ITEM", "doc", '{"a":1,"b":{"c":2}}'))
    assert cache.get_local("doc") == {"a": 1, "b": {"c": 2}}


def test_patch_item_deep_merges_instead_of_overwriting(cache):
    cache._update_local_cache(_event("ADD_ITEM", "doc", '{"a":1,"b":{"c":2}}'))
    cache._update_local_cache(_event("PATCH_ITEM", "doc", '{"b":{"d":3}}'))
    assert cache.get_local("doc") == {"a": 1, "b": {"c": 2, "d": 3}}


def test_patch_null_deletes_field(cache):
    cache._update_local_cache(_event("ADD_ITEM", "doc", '{"a":1,"b":2}'))
    cache._update_local_cache(_event("PATCH_ITEM", "doc", '{"b":null}'))
    assert cache.get_local("doc") == {"a": 1}


def test_patch_on_absent_key_starts_from_delta(cache):
    cache._update_local_cache(_event("PATCH_ITEM", "doc", '{"a":1}'))
    assert cache.get_local("doc") == {"a": 1}


def test_scalars_and_lists_replace(cache):
    cache._update_local_cache(_event("ADD_ITEM", "doc", '{"n":1,"list":[1,2,3]}'))
    cache._update_local_cache(_event("PATCH_ITEM", "doc", '{"n":9,"list":[4]}'))
    assert cache.get_local("doc") == {"n": 9, "list": [4]}


def test_remove_and_clear(cache):
    cache._update_local_cache(_event("ADD_ITEM", "doc", '{"a":1}'))
    cache._update_local_cache(_event("REMOVE_ITEM", "doc"))
    assert cache.get_local("doc") is None

    cache._update_local_cache(_event("ADD_ITEM", "doc", '{"a":1}'))
    cache._update_local_cache(_event("CLEAR_CACHE"))
    assert cache.get_local("doc") is None


def test_deep_merge_does_not_mutate_inputs():
    target = {"a": 1, "b": {"c": 2}}
    patch = {"b": {"d": 3}}
    merged = _deep_merge(target, patch)
    assert merged == {"a": 1, "b": {"c": 2, "d": 3}}
    assert target == {"a": 1, "b": {"c": 2}}


@pytest.mark.asyncio
async def test_patch_async_delegates(cache, client_mock):
    client_mock.patch_item_async = AsyncMock()
    await cache.patch_async("doc", {"b": 2})
    client_mock.patch_item_async.assert_awaited_once_with("test-cache", "doc", '{"b": 2}')
