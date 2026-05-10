use aeron_cache_embedded_client::{AeronCacheClient};
use mockito::Server;

// In Rust, mocking structs like `AeronCacheClient` directly is non-trivial without a trait, 
// so we typically fall back to testing the EmbeddedAeronCache together with the mock server.
// This tests that calling `insert` on the embedded cache correctly routes via the client 
// to the external Mock server endpoint.

#[test]
fn test_embedded_cache_put_delegates() {
    let mut server = Server::new();
    let mock = server.mock("POST", "/api/v1/cache/test-cache")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-cache", "key": "key1", "status": "OK", "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let cache = client.get_cache("test-cache");
    
    let result = cache.insert("key1", "val1").unwrap();
    assert_eq!(result.key, "key1");
    mock.assert();
}

#[test]
fn test_embedded_cache_removes_and_clears_delegates() {
    let mut server = Server::new();
    let mock_del = server.mock("DELETE", "/api/v1/cache/test-cache/key1")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-cache", "key": "key1", "operationStatus": "SUCCESS"}"#)
        .create();
        
    let mock_clear = server.mock("DELETE", "/api/v1/cache/test-cache")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-cache", "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let cache = client.get_cache("test-cache");
    
    cache.remove("key1").unwrap();
    mock_del.assert();

    cache.clear().unwrap();
    mock_clear.assert();
}
