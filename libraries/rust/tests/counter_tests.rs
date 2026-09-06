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
