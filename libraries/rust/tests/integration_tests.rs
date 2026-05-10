use aeron_cache_embedded_client::{AeronCacheClient};
use std::env;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn generate_id(prefix: &str) -> String {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros();
    format!("{}-{}", prefix, ts)
}

// Run integration tests only when AERON_CACHE_BASE_URL is present
fn get_urls() -> Option<(String, String)> {
    let base_url = env::var("AERON_CACHE_BASE_URL").ok()?;
    let ws_url = env::var("AERON_CACHE_WS_URL")
        .unwrap_or_else(|_| base_url.replace("http://", "ws://").replace("https://", "wss://"));
    Some((base_url, ws_url))
}

#[test]
fn test_integration_cache_operations() {
    let Some((base_url, ws_url)) = get_urls() else {
        println!("Skipping test_integration_cache_operations: AERON_CACHE_BASE_URL not set");
        return;
    };

    let client = AeronCacheClient::new(base_url, ws_url);
    let cache_id = generate_id("it-cache");

    let create_resp = client.create_cache(&cache_id).expect("Failed to create cache");
    assert_eq!(create_resp.cache_id, cache_id);

    let cache = client.get_cache(&cache_id);

    // Put an item
    let put_resp = cache.insert("key1", "val1").unwrap();
    assert_eq!(put_resp.key, "key1");

    // Get the item
    let get_resp = cache.get("key1").unwrap();
    assert_eq!(get_resp.value, "val1");

    // Remove the item
    cache.remove("key1").unwrap();

    // Get the item again, should handle 404 cleanly
    let get_resp2 = cache.get("key1").unwrap();
    assert!(get_resp2.operation_status == "UNKNOWN_KEY" || get_resp2.value.is_empty());
}

#[test]
fn test_integration_websocket_subscription() {
    let Some((base_url, ws_url)) = get_urls() else {
        println!("Skipping test_integration_websocket_subscription: AERON_CACHE_BASE_URL not set");
        return;
    };

    let client = AeronCacheClient::new(base_url, ws_url);
    let cache_id = generate_id("it-ws");
    
    client.create_cache(&cache_id).expect("Failed to create cache");
    let cache = client.get_cache(&cache_id);

    // Give websocket a moment to connect if it happens internally, but we must explicitly poll in Rust!
    // We subscribe first to set up the connection.
    let mut ws = cache.subscribe().expect("Failed to subscribe");

    // Put standard delay
    thread::sleep(Duration::from_secs(1));

    cache.insert("ws-key", "ws-val").unwrap();

    // Rust websocket implementation might be blocking during read.
    // Read one message from websocket stream.
    ws.read_message().expect("Failed to read message");
    
    // Usually it starts with some message or our ADD_ITEM will arrive.
    // In our rudimentary WebSocket impl, it parses incoming to cache immediately inside `read_message`.
    // We check if cache has updated.
    
    // Attempt local cache assertion directly, since read_message should process and store it.
    let local_val = cache.get_local("ws-key");
    assert_eq!(local_val, Some("ws-val".to_string()));

    cache.clear().unwrap();
}