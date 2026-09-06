use aeron_cache_embedded_client::{AeronCacheClient};
use mockito::Server;

// EmbeddedCounterCache is tested against a mock HTTP server, verifying its methods
// route through the client to the correct counter endpoints.

#[test]
fn test_embedded_counter_put_and_increment_delegate() {
    let mut server = Server::new();
    let put_mock = server.mock("POST", "/api/v1/counters/test-counter")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "key": "hits", "operationStatus": "SUCCESS"}"#)
        .create();

    let inc_mock = server.mock("POST", "/api/v1/counters/increment/test-counter")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "key": "hits", "value": 15, "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let counters = client.get_counter_cache("test-counter");

    let put = counters.insert("hits", 10).unwrap();
    assert_eq!(put.key, "hits");
    put_mock.assert();

    let inc = counters.increment("hits", 5).unwrap();
    assert_eq!(inc.value, 15);
    inc_mock.assert();
}

#[test]
fn test_embedded_counter_remove_and_clear_delegate() {
    let mut server = Server::new();
    let del_mock = server.mock("DELETE", "/api/v1/counters/test-counter/hits")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "key": "hits", "operationStatus": "SUCCESS"}"#)
        .create();

    let clear_mock = server.mock("DELETE", "/api/v1/counters/test-counter")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cacheId": "test-counter", "operationStatus": "SUCCESS"}"#)
        .create();

    let client = AeronCacheClient::new(server.url(), "ws://localhost:7071".to_string());
    let counters = client.get_counter_cache("test-counter");

    counters.remove("hits").unwrap();
    del_mock.assert();

    counters.clear().unwrap();
    clear_mock.assert();
}
