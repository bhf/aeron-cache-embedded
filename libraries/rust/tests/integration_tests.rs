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
    // We might need to skip some initial messages if there are any, 
    // or wait for the ADD_ITEM specifically.
    let mut found = false;
    for _ in 0..5 {
        if let Ok(msg) = ws.read_message() {
            if let tungstenite::Message::Text(text) = msg {
                if text.contains("ws-key") && text.contains("ADD_ITEM") {
                    found = true;
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    assert!(found, "Should have received ADD_ITEM for ws-key through websocket");
    
    // Attempt local cache assertion directly, since read_message should process and store it.
    let local_val = cache.get_local("ws-key");
    assert_eq!(local_val, Some("ws-val".to_string()));

    cache.clear().unwrap();
}

#[test]
fn test_integration_websocket_hydration() {
    let Some((base_url, ws_url)) = get_urls() else {
        println!("Skipping test_integration_websocket_hydration: AERON_CACHE_BASE_URL not set");
        return;
    };

    let client = AeronCacheClient::new(base_url, ws_url);
    let cache_id = generate_id("it-hydrate");
    
    client.create_cache(&cache_id).expect("Failed to create cache");
    let pre_fill = client.get_cache(&cache_id);
    pre_fill.insert("hydrate-key", "hydrate-val").unwrap();

    let cache = client.get_cache(&cache_id);
    let mut ws = cache.subscribe_ext(true).expect("Failed to subscribe with hydration");

    // Rust needs manual polling. Hydration sends existing items.
    // Usually one message per item or a batch. 
    // We poll and look for our key.
    let mut found = false;
    for _ in 0..10 {
        if let Ok(msg) = ws.read_message() {
            if let tungstenite::Message::Text(text) = msg {
                if text.contains("hydrate-key") && text.contains("ADD_ITEM") {
                    found = true;
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    assert!(found, "Should have received hydrated ADD_ITEM for hydrate-key");

    let local_val = cache.get_local("hydrate-key");
    assert_eq!(local_val, Some("hydrate-val".to_string()));

    cache.clear().unwrap();
}

#[test]
fn test_get_and_clear_cache() {
    let Some((base_url, ws_url)) = get_urls() else {
        println!("Skipping test_get_and_clear_cache: AERON_CACHE_BASE_URL not set");
        return;
    };

    let client = AeronCacheClient::new(base_url, ws_url);
    let cache_id = generate_id("it-cache2");

    client.create_cache(&cache_id).unwrap();
    client.put_item(&cache_id, "key1", "val1").unwrap();
    client.put_item(&cache_id, "key2", "val2").unwrap();

    let get_resp = client.get_cache_items(&cache_id).unwrap();
    assert_eq!(get_resp.items.len(), 2);

    let clear_resp = client.clear_cache(&cache_id).unwrap();
    assert_eq!(clear_resp.operation_status, "SUCCESS");

    let get_resp2 = client.get_cache_items(&cache_id).unwrap();
    assert_eq!(get_resp2.items.len(), 0);
}

#[test]
fn test_integration_bulk_operations() {
    use aeron_cache_embedded_client::{BulkCacheOpsRequest, CacheOperationRequest, BulkOperationType};

    let Some((base_url, ws_url)) = get_urls() else {
        println!("Skipping test_integration_bulk_operations: AERON_CACHE_BASE_URL not set");
        return;
    };

    let client = AeronCacheClient::new(base_url, ws_url);
    let cache_id = generate_id("it-bulk");
    let req_id = generate_id("req");

    let request = BulkCacheOpsRequest {
        request_id: req_id.clone(),
        operations: vec![
            CacheOperationRequest {
                operation_type: BulkOperationType::CreateCache,
                request_id: "op-1".to_string(),
                cache_id: cache_id.clone(),
                key: None,
                value: None,
                ttl: None,
            },
            CacheOperationRequest {
                operation_type: BulkOperationType::AddItem,
                request_id: "op-2".to_string(),
                cache_id: cache_id.clone(),
                key: Some("bulk-key".to_string()),
                value: Some("bulk-val".to_string()),
                ttl: None,
            },
            CacheOperationRequest {
                operation_type: BulkOperationType::GetItem,
                request_id: "op-3".to_string(),
                cache_id: cache_id.clone(),
                key: Some("bulk-key".to_string()),
                value: None,
                ttl: None,
            },
        ],
    };

    let response = client.bulk_ops(&request).expect("Failed bulk operations");
    assert_eq!(response.request_id, req_id);
    assert_eq!(response.operation_responses.len(), 3);

    // Verify GET_ITEM result
    assert_eq!(response.operation_responses[2].request_id, "op-3");
    assert_eq!(response.operation_responses[2].value, Some("bulk-val".to_string()));
}


#[test]
fn test_integration_put_timed_item() {
    let Some((base_url, ws_url)) = get_urls() else {
        println!("Skipping test_integration_put_timed_item: AERON_CACHE_BASE_URL not set");
        return;
    };

    let client = AeronCacheClient::new(base_url, ws_url);
    let cache_id = generate_id("it-timed");

    client.create_cache(&cache_id).expect("Failed to create cache");
    let cache = client.get_cache(&cache_id);

    // Put a timed item with 2 second TTL (2000 ms)
    let put_resp = cache.insert_timed("timed-key", "timed-val", 2000).expect("Failed to put timed item");
    assert_eq!(put_resp.key, "timed-key");

    // Get immediately - should exist
    let get_resp = cache.get("timed-key").expect("Failed to get item");
    assert_eq!(get_resp.value, "timed-val");

    // Wait for TTL to expire (3 seconds)
    thread::sleep(Duration::from_secs(3));

    // Get again - should be gone
    let get_resp2 = cache.get("timed-key").expect("Failed to get item after expiry");
    assert!(get_resp2.operation_status == "UNKNOWN_KEY" || get_resp2.value.is_empty());
}

