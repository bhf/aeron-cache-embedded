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
