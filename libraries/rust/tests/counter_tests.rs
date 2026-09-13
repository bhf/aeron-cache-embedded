use aeron_cache_embedded_client::{AeronCacheClient, BulkCacheOpsRequest, CacheOperationRequest, BulkOperationType};
use mockito::Server;

#[test]
fn test_create_counter_cache() {
    let mut server = Server::new();
    let mock = server.mock("POST", "/api/v1/counters/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let result = client.create_counter_cache("test-counter").unwrap();
    assert_eq!(result.cache_id, "test-counter");
    mock.assert();
}

#[test]
fn test_get_counter() {
    let mut server = Server::new();
    let mock = server.mock("GET", "/api/v1/counters/test-counter/hits")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "key": "hits", "value": 42, "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let result = client.get_counter("test-counter", "hits").unwrap();
    assert_eq!(result.value, 42);
    mock.assert();
}

#[test]
fn test_put_timed_counter() {
    let mut server = Server::new();
    let mock = server.mock("POST", "/api/v1/counters/timed/test-counter")
        .match_body(mockito::Matcher::PartialJsonString(r#"{"key":"hits","value":42,"ttl":1000}"#.to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "key": "hits", "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let result = client.put_timed_counter("test-counter", "hits", 42, 1000).unwrap();
    assert_eq!(result.key, "hits");
    mock.assert();
}

#[test]
fn test_increment_counter() {
    let mut server = Server::new();
    let mock = server.mock("POST", "/api/v1/counters/increment/test-counter")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "key": "hits", "value": 43, "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let result = client.increment_counter("test-counter", "hits", 1).unwrap();
    assert_eq!(result.value, 43);
    mock.assert();
}

#[test]
fn test_decrement_counter() {
    let mut server = Server::new();
    let mock = server.mock("POST", "/api/v1/counters/decrement/test-counter")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "key": "hits", "value": 41, "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let result = client.decrement_counter("test-counter", "hits", 1).unwrap();
    assert_eq!(result.value, 41);
    mock.assert();
}

#[test]
fn test_set_counter() {
    let mut server = Server::new();
    let mock = server.mock("POST", "/api/v1/counters/set/test-counter")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "key": "hits", "value": 100, "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let result = client.set_counter("test-counter", "hits", 100).unwrap();
    assert_eq!(result.value, 100);
    mock.assert();
}

#[test]
fn test_counter_server_error_throws() {
    let mut server = Server::new();
    let mock = server.mock("GET", "/api/v1/counters/test-counter/hits")
        .with_status(500)
        .with_body("Internal Server Error")
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let result = client.get_counter("test-counter", "hits");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("500"));
    mock.assert();
}

#[test]
fn test_get_counter_items() {
    let mut server = Server::new();
    let mock = server.mock("GET", "/api/v1/counters/test-counter")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "operationStatus": "SUCCESS", "items": [{"key": "a", "value": 1}, {"key": "b", "value": 2}]}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let result = client.get_counter_items("test-counter").unwrap();
    assert_eq!(result.cache_id, "test-counter");
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.items[0].key, "a");
    assert_eq!(result.items[0].value, 1);
    assert_eq!(result.items[1].value, 2);
    mock.assert();
}

#[test]
fn test_clear_counter_cache() {
    let mut server = Server::new();
    let mock = server.mock("PATCH", "/api/v1/counters/test-counter")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let result = client.clear_counter_cache("test-counter").unwrap();
    assert_eq!(result.cache_id, "test-counter");
    assert_eq!(result.operation_status, "SUCCESS");
    mock.assert();
}

#[test]
fn test_cancel_counter_item_removal() {
    let mut server = Server::new();
    let mock = server.mock("POST", "/api/v1/counters/test-counter/hits/cancel-removal")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "key": "hits", "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let result = client.cancel_counter_item_removal("test-counter", "hits").unwrap();
    assert_eq!(result.cache_id, "test-counter");
    assert_eq!(result.key, "hits");
    assert_eq!(result.operation_status, "SUCCESS");
    mock.assert();
}

#[test]
fn test_get_counter_caches() {
    let mut server = Server::new();
    let mock = server.mock("GET", "/api/v1/counters-caches")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"cacheId": "cc1", "itemCount": 2}, {"cacheId": "cc2", "itemCount": 0}]"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let caches = client.get_counter_caches().unwrap();
    assert_eq!(caches.len(), 2);
    assert_eq!(caches[0].cache_id, "cc1");
    assert_eq!(caches[0].item_count, 2);
    mock.assert();
}

#[test]
fn test_get_counter_stats() {
    let mut server = Server::new();
    let mock = server.mock("GET", "/api/v1/counters-stats")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"totalOpsCount": 7, "totalCachesCount": 1, "totalItemsCount": 3, "errorCount": 0}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let stats = client.get_counter_stats().unwrap();
    assert_eq!(stats.total_ops_count, 7);
    assert_eq!(stats.total_caches_count, 1);
    assert_eq!(stats.total_items_count, 3);
    assert_eq!(stats.error_count, 0);
    mock.assert();
}

#[test]
fn test_bulk_ops_with_counter() {
    let mut server = Server::new();
    let mock = server.mock("POST", "/api/v1/cache/bulkops")
        .match_body(mockito::Matcher::PartialJsonString(
            r#"{"operations":[{"operationType":"INCREMENT_COUNTER","counterValue":5}]}"#.to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"requestId": "req-1", "operationResponses": [
            {"requestId": "op-1", "status": "SUCCESS", "cacheId": "test-counter", "key": "hits", "value": "5"}
        ]}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());

    let request = BulkCacheOpsRequest {
        request_id: "req-1".to_string(),
        operations: vec![
            CacheOperationRequest {
                operation_type: BulkOperationType::IncrementCounter,
                request_id: "op-1".to_string(),
                cache_id: "test-counter".to_string(),
                key: Some("hits".to_string()),
                value: None,
                ttl: None,
                counter_value: Some(5),
            },
        ],
    };

    let result = client.bulk_ops(&request).unwrap();
    assert_eq!(result.request_id, "req-1");
    assert_eq!(result.operation_responses.len(), 1);
    assert_eq!(result.operation_responses[0].value.as_deref(), Some("5"));
    mock.assert();
}
