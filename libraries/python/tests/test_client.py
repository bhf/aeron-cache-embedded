import json
import pytest
import responses
from requests.exceptions import HTTPError
from aeron_cache.client import AeronCacheClient

@pytest.fixture
def client():
    return AeronCacheClient("http://localhost:7070", "ws://localhost:7071")

@responses.activate
def test_create_cache(client):
    responses.add(
        responses.POST,
        "http://localhost:7070/api/v1/cache",
        json={"cacheId": "test-cache", "operationStatus": "SUCCESS"},
        status=200
    )
    
    response = client.create_cache("test-cache")
    assert response.cacheId == "test-cache"
    assert response.operationStatus == "SUCCESS"

@responses.activate
def test_put_item(client):
    responses.add(
        responses.POST,
        "http://localhost:7070/api/v1/cache/test-cache",
        json={"cacheId": "test-cache", "key": "my-key", "operationStatus": "SUCCESS"},
        status=200
    )
    
    response = client.put_item("test-cache", "my-key", "my-value")
    assert response.cacheId == "test-cache"
    assert response.key == "my-key"

@responses.activate
def test_get_item(client):
    responses.add(
        responses.GET,
        "http://localhost:7070/api/v1/cache/test-cache/my-key",
        json={"cacheId": "test-cache", "key": "my-key", "value": "my-value", "operationStatus": "SUCCESS"},
        status=200
    )
    
    response = client.get_item("test-cache", "my-key")
    assert response.value == "my-value"
    assert response.key == "my-key"

@responses.activate
def test_delete_item(client):
    responses.add(
        responses.DELETE,
        "http://localhost:7070/api/v1/cache/test-cache/my-key",
        json={"cacheId": "test-cache", "key": "my-key", "operationStatus": "SUCCESS"},
        status=200
    )
    
    response = client.delete_item("test-cache", "my-key")
    assert response.key == "my-key"

@responses.activate
def test_delete_cache(client):
    responses.add(
        responses.DELETE,
        "http://localhost:7070/api/v1/cache/test-cache",
        json={"cacheId": "test-cache", "operationStatus": "SUCCESS"},
        status=200
    )
    
    response = client.delete_cache("test-cache")
    assert response.cacheId == "test-cache"

@responses.activate
def test_bulk_ops(client):
    from aeron_cache.models import BulkCacheOpsRequest, CacheOperationRequest, BulkOperationType
    
    responses.add(
        responses.POST,
        "http://localhost:7070/api/v1/cache/bulkops",
        json={
            "requestId": "req-1",
            "operationResponses": [
                {"requestId": "op-1", "status": "SUCCESS", "cacheId": "test-cache", "key": "k1"}
            ]
        },
        status=200
    )
    
    request = BulkCacheOpsRequest(
        requestId="req-1",
        operations=[
            CacheOperationRequest(
                operationType=BulkOperationType.ADD_ITEM,
                requestId="op-1",
                cacheId="test-cache",
                key="k1",
                value="v1"
            )
        ]
    )
    
    response = client.bulk_ops(request)
    assert response.requestId == "req-1"
    assert len(response.operationResponses) == 1
    assert response.operationResponses[0].status == "SUCCESS"

@responses.activate
def test_http_error_throws_exception(client):
    responses.add(
        responses.GET,
        "http://localhost:7070/api/v1/cache/test-cache/my-key",
        body="Internal Server Error",
        status=500
    )
    
    with pytest.raises(HTTPError):
        client.get_item("test-cache", "my-key")

@responses.activate
def test_allow_business_logic_http_errors(client):
    responses.add(
        responses.GET,
        "http://localhost:7070/api/v1/cache/test-cache/non-existent-key",
        json={"operationStatus": "UNKNOWN_KEY"},
        status=404
    )
    
    response = client.get_item("test-cache", "non-existent-key")
    assert response.operationStatus == "UNKNOWN_KEY"

@responses.activate
def test_get_cache(client):
    responses.add(
        responses.GET,
        "http://localhost:7070/api/v1/cache/test-cache",
        json={"cacheId": "test-cache", "operationStatus": "SUCCESS", "items": [{"key": "my-key", "value": "my-value"}]},
        status=200
    )
    
    response = client.get_cache_items("test-cache")
    assert response.cacheId == "test-cache"
    assert len(response.items) == 1
    assert response.items[0].key == "my-key"
    assert response.items[0].value == "my-value"

@responses.activate
def test_clear_cache(client):
    responses.add(
        responses.PATCH,
        "http://localhost:7070/api/v1/cache/test-cache",
        json={"cacheId": "test-cache", "operationStatus": "SUCCESS"},
        status=200
    )
    
    response = client.clear_cache("test-cache")
    assert response.cacheId == "test-cache"
    assert response.operationStatus == "SUCCESS"

@responses.activate
def test_put_timed_item(client):
    responses.add(
        responses.POST,
        "http://localhost:7070/api/v1/cache/timed/test-cache",
        json={"cacheId": "test-cache", "key": "my-key", "operationStatus": "SUCCESS"},
        status=200
    )
    
    response = client.put_timed_item("test-cache", "my-key", "my-value", 1000)
    assert response.cacheId == "test-cache"
    assert response.operationStatus == "SUCCESS"

@responses.activate
def test_patch_item(client):
    responses.add(
        responses.PATCH,
        "http://localhost:7070/api/v1/cache/test-cache/my-key",
        json={"cacheId": "test-cache", "key": "my-key", "operationStatus": "SUCCESS"},
        status=200
    )

    response = client.patch_item("test-cache", "my-key", '{"field":"newValue"}')
    assert response.cacheId == "test-cache"
    assert response.key == "my-key"
    assert response.operationStatus == "SUCCESS"
    body = json.loads(responses.calls[0].request.body)
    assert body == {"value": '{"field":"newValue"}'}

@responses.activate
def test_cancel_item_removal(client):
    responses.add(
        responses.POST,
        "http://localhost:7070/api/v1/cache/test-cache/my-key/cancel-removal",
        json={"cacheId": "test-cache", "key": "my-key", "operationStatus": "SUCCESS"},
        status=200
    )

    response = client.cancel_item_removal("test-cache", "my-key")
    assert response.cacheId == "test-cache"
    assert response.key == "my-key"
    assert response.operationStatus == "SUCCESS"

@responses.activate
def test_get_caches(client):
    responses.add(
        responses.GET,
        "http://localhost:7070/api/v1/caches",
        json=[{"cacheId": "c1", "itemCount": 3}, {"cacheId": "c2", "itemCount": 0}],
        status=200
    )

    caches = client.get_caches()
    assert len(caches) == 2
    assert caches[0].cacheId == "c1"
    assert caches[0].itemCount == 3
    assert caches[1].cacheId == "c2"

@responses.activate
def test_get_stats(client):
    responses.add(
        responses.GET,
        "http://localhost:7070/api/v1/stats",
        json={"totalOpsCount": 10, "totalCachesCount": 2, "totalItemsCount": 5, "errorCount": 1},
        status=200
    )

    stats = client.get_stats()
    assert stats.totalOpsCount == 10
    assert stats.totalCachesCount == 2
    assert stats.totalItemsCount == 5
    assert stats.errorCount == 1

def test_ws_query_builder():
    assert AeronCacheClient._ws_query() == ""
    assert AeronCacheClient._ws_query(keys="a,b") == "?keys=a%2Cb"
    assert AeronCacheClient._ws_query(keys=["c1:k1", "k2"]) == "?keys=c1%3Ak1%2Ck2"
    assert AeronCacheClient._ws_query(mode="patch") == "?mode=patch"
    q = AeronCacheClient._ws_query(keys="k", mode="full")
    assert q.startswith("?") and "keys=k" in q and "mode=full" in q
