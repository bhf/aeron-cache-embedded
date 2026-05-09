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
