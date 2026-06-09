use aeron_cache_embedded_client::{AeronCacheClient};
use mockito::Server;

#[test]
fn test_create_cache_success() {
    let mut server = Server::new();
    let mock = server.mock("POST", "/api/v1/cache")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-cache", "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    
    let result = client.create_cache("test-cache").unwrap();
    assert_eq!(result.cache_id, "test-cache");
    assert_eq!(result.operation_status, "SUCCESS");
    
    mock.assert();
}

#[test]
fn test_get_item_success() {
    let mut server = Server::new();
    let mock = server.mock("GET", "/api/v1/cache/test-cache/my-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-cache", "key": "my-key", "value": "my-value", "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    
    let result = client.get_item("test-cache", "my-key").unwrap();
    assert_eq!(result.value, "my-value");
    
    mock.assert();
}

#[test]
fn test_server_error_throws() {
    let mut server = Server::new();
    let mock = server.mock("GET", "/api/v1/cache/test-cache/my-key")
        .with_status(500)
        .with_body("Internal Server Error")
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    
    let result = client.get_item("test-cache", "my-key");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("500"));
    
    mock.assert();
}

#[test]
fn test_business_error_404_allowed() {
    let mut server = Server::new();
    let mock = server.mock("GET", "/api/v1/cache/test-cache/non-existent-key")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-cache", "key": "non-existent-key", "value": "", "operationStatus": "UNKNOWN_KEY"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    
    let result = client.get_item("test-cache", "non-existent-key").unwrap();
    assert_eq!(result.operation_status, "UNKNOWN_KEY");
    
    mock.assert();
}

#[test]
fn test_get_cache_items() {
    let mut server = mockito::Server::new();
    let url = server.url();

    let _m = server.mock("GET", "/api/v1/cache/test-cache-get")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId":"test-cache-get","operationStatus":"SUCCESS","items":[{"key":"k1","value":"v1"}]}"#)
        .create();

    let client = AeronCacheClient::new(url, "ws://localhost".into());
    let response = client.get_cache_items("test-cache-get").unwrap();

    assert_eq!(response.cache_id, "test-cache-get");
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].key, "k1");
}

#[test]
fn test_clear_cache() {
    let mut server = mockito::Server::new();
    let url = server.url();

    let _m = server.mock("PATCH", "/api/v1/cache/test-cache-clear")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId":"test-cache-clear","operationStatus":"SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(url, "ws://localhost".into());
    let response = client.clear_cache("test-cache-clear").unwrap();

    assert_eq!(response.operation_status, "SUCCESS");
}

#[test]
fn test_bulk_ops() {
    use aeron_cache_embedded_client::{BulkCacheOpsRequest, CacheOperationRequest, BulkOperationType};

    let mut server = mockito::Server::new();
    let url = server.url();

    let _m = server.mock("POST", "/api/v1/cache/bulkops")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"requestId": "req-1", "operationResponses": [{"requestId": "op-1", "status": "SUCCESS", "cacheId": "test-cache", "key": "k1"}]}"#)
        .create();

    let client = AeronCacheClient::new(url, "ws://localhost".into());
    
    let request = BulkCacheOpsRequest {
        request_id: "req-1".to_string(),
        operations: vec![
            CacheOperationRequest {
                operation_type: BulkOperationType::AddItem,
                request_id: "op-1".to_string(),
                cache_id: "test-cache".to_string(),
                key: Some("k1".to_string()),
                value: Some("v1".to_string()),
                ttl: None,
            }
        ],
    };

    let response = client.bulk_ops(&request).unwrap();
    assert_eq!(response.request_id, "req-1");
    assert_eq!(response.operation_responses.len(), 1);
    assert_eq!(response.operation_responses[0].status, "SUCCESS");
}

#[test]
fn test_put_timed_item() {
    let mut server = mockito::Server::new();
    let url = server.url();

    let _m = server.mock("POST", "/api/v1/cache/timed/test-cache")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-cache", "key": "k1", "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(url, "ws://localhost".into());
    let response = client.put_timed_item("test-cache", "k1", "v1", 1000).unwrap();

    assert_eq!(response.operation_status, "SUCCESS");
}
