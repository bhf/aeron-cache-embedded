import json
import pytest
import responses
from requests.exceptions import HTTPError
from aeron_cache.client import AeronCacheClient
from aeron_cache.models import (
    BulkCacheOpsRequest,
    CacheOperationRequest,
    BulkOperationType,
)

@pytest.fixture
def client():
    return AeronCacheClient("http://localhost:7070", "ws://localhost:7071")

@responses.activate
def test_create_counter_cache(client):
    responses.add(responses.POST, "http://localhost:7070/api/v1/counters/",
                  json={"cacheId": "test-counter", "operationStatus": "SUCCESS"}, status=200)
    response = client.create_counter_cache("test-counter")
    assert response.cacheId == "test-counter"
    assert response.operationStatus == "SUCCESS"

@responses.activate
def test_put_counter(client):
    responses.add(responses.POST, "http://localhost:7070/api/v1/counters/test-counter",
                  json={"cacheId": "test-counter", "key": "hits", "operationStatus": "SUCCESS"}, status=200)
    response = client.put_counter("test-counter", "hits", 42)
    assert response.cacheId == "test-counter"
    assert response.key == "hits"

@responses.activate
def test_put_timed_counter(client):
    responses.add(responses.POST, "http://localhost:7070/api/v1/counters/timed/test-counter",
                  json={"cacheId": "test-counter", "key": "hits", "operationStatus": "SUCCESS"}, status=200)
    response = client.put_timed_counter("test-counter", "hits", 42, 1000)
    assert response.operationStatus == "SUCCESS"
    body = json.loads(responses.calls[0].request.body)
    assert body == {"key": "hits", "value": 42, "ttl": 1000}

@responses.activate
def test_get_counter(client):
    responses.add(responses.GET, "http://localhost:7070/api/v1/counters/test-counter/hits",
                  json={"cacheId": "test-counter", "key": "hits", "value": 42, "operationStatus": "SUCCESS"}, status=200)
    response = client.get_counter("test-counter", "hits")
    assert response.value == 42
    assert response.key == "hits"

@responses.activate
def test_increment_counter(client):
    responses.add(responses.POST, "http://localhost:7070/api/v1/counters/increment/test-counter",
                  json={"cacheId": "test-counter", "key": "hits", "value": 43, "operationStatus": "SUCCESS"}, status=200)
    response = client.increment_counter("test-counter", "hits", 1)
    assert response.value == 43

@responses.activate
def test_decrement_counter(client):
    responses.add(responses.POST, "http://localhost:7070/api/v1/counters/decrement/test-counter",
                  json={"cacheId": "test-counter", "key": "hits", "value": 41, "operationStatus": "SUCCESS"}, status=200)
    response = client.decrement_counter("test-counter", "hits", 1)
    assert response.value == 41

@responses.activate
def test_set_counter(client):
    responses.add(responses.POST, "http://localhost:7070/api/v1/counters/set/test-counter",
                  json={"cacheId": "test-counter", "key": "hits", "value": 100, "operationStatus": "SUCCESS"}, status=200)
    response = client.set_counter("test-counter", "hits", 100)
    assert response.value == 100

@responses.activate
def test_delete_counter(client):
    responses.add(responses.DELETE, "http://localhost:7070/api/v1/counters/test-counter/hits",
                  json={"cacheId": "test-counter", "key": "hits", "operationStatus": "SUCCESS"}, status=200)
    response = client.delete_counter("test-counter", "hits")
    assert response.key == "hits"

@responses.activate
def test_delete_counter_cache(client):
    responses.add(responses.DELETE, "http://localhost:7070/api/v1/counters/test-counter",
                  json={"cacheId": "test-counter", "operationStatus": "SUCCESS"}, status=200)
    response = client.delete_counter_cache("test-counter")
    assert response.cacheId == "test-counter"

@responses.activate
def test_counter_http_error_throws(client):
    responses.add(responses.GET, "http://localhost:7070/api/v1/counters/test-counter/hits",
                  body="Internal Server Error", status=500)
    with pytest.raises(HTTPError):
        client.get_counter("test-counter", "hits")

@responses.activate
def test_bulk_ops_with_counter(client):
    responses.add(
        responses.POST,
        "http://localhost:7070/api/v1/cache/bulkops",
        json={
            "requestId": "req-1",
            "operationResponses": [
                {"requestId": "op-1", "status": "SUCCESS", "cacheId": "test-counter"},
                {"requestId": "op-2", "status": "SUCCESS", "cacheId": "test-counter", "key": "hits", "value": "5"},
            ],
        },
        status=200
    )

    request = BulkCacheOpsRequest(
        requestId="req-1",
        operations=[
            CacheOperationRequest(operationType=BulkOperationType.CREATE_COUNTER_CACHE, requestId="op-1", cacheId="test-counter"),
            CacheOperationRequest(operationType=BulkOperationType.INCREMENT_COUNTER, requestId="op-2", cacheId="test-counter", key="hits", counterValue=5),
        ],
    )
    response = client.bulk_ops(request)
    assert response.requestId == "req-1"
    assert len(response.operationResponses) == 2
    assert response.operationResponses[1].value == "5"

    # Counter op should serialize operationType as a plain string and include counterValue; unset fields omitted.
    sent = json.loads(responses.calls[0].request.body)
    assert sent["operations"][0]["operationType"] == "CREATE_COUNTER_CACHE"
    assert sent["operations"][1]["counterValue"] == 5
    assert "value" not in sent["operations"][1]
    assert "ttl" not in sent["operations"][1]
