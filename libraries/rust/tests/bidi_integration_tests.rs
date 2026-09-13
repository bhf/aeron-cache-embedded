//! End-to-end tests for the bidirectional WebSocket transport ([`AeronBidiClient`]) against a live
//! backend. Gated exactly like the HTTP integration tests: they run only when `AERON_CACHE_BASE_URL`
//! is set, deriving the WebSocket URL from `AERON_CACHE_WS_URL` (or from the base URL if unset).
//!
//! Mirrors the Python bidi integration tests (`libraries/python/tests/test_bidi_integration.py`).

use aeron_cache_embedded_client::AeronBidiClient;
use std::env;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn ws_url() -> Option<String> {
    // Gate on AERON_CACHE_BASE_URL like the other integration tests.
    let base_url = env::var("AERON_CACHE_BASE_URL").ok()?;
    let url = env::var("AERON_CACHE_WS_URL")
        .unwrap_or_else(|_| base_url.replace("http://", "ws://").replace("https://", "wss://"));
    Some(url)
}

fn unique(prefix: &str) -> String {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("{prefix}-{ts:x}")
}

#[test]
fn bidi_cache_lifecycle() {
    let Some(url) = ws_url() else {
        println!("Skipping bidi_cache_lifecycle: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-cache");

    assert_eq!(client.create_cache(&cache_id).unwrap().cache_id, cache_id);
    assert_eq!(client.put_item(&cache_id, "k1", "v1").unwrap().operation_status, "SUCCESS");
    assert_eq!(client.get_item(&cache_id, "k1").unwrap().value, "v1");

    client.put_item(&cache_id, "doc", "{\"a\":1}").unwrap();
    client.patch_item(&cache_id, "doc", "{\"b\":2}").unwrap();
    let doc = client.get_item(&cache_id, "doc").unwrap().value;
    assert!(doc.contains("\"a\":1") && doc.contains("\"b\":2"), "patched doc was {doc}");

    let items = client.get_cache_items(&cache_id).unwrap().items;
    let keys: std::collections::HashSet<_> = items.iter().map(|i| i.key.clone()).collect();
    assert_eq!(keys, ["k1".to_string(), "doc".to_string()].into_iter().collect());

    assert_eq!(client.delete_item(&cache_id, "k1").unwrap().operation_status, "SUCCESS");
    assert_eq!(client.clear_cache(&cache_id).unwrap().operation_status, "SUCCESS");
    client.delete_cache(&cache_id).unwrap();
}

#[test]
fn bidi_counter_lifecycle() {
    let Some(url) = ws_url() else {
        println!("Skipping bidi_counter_lifecycle: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-counter");

    client.create_counter_cache(&cache_id).unwrap();
    client.put_counter(&cache_id, "hits", 10).unwrap();
    assert_eq!(client.increment_counter(&cache_id, "hits", 5).unwrap().value, 15);
    assert_eq!(client.decrement_counter(&cache_id, "hits", 3).unwrap().value, 12);
    assert_eq!(client.set_counter(&cache_id, "hits", 100).unwrap().value, 100);
    assert_eq!(client.get_counter(&cache_id, "hits").unwrap().value, 100);

    client.put_counter(&cache_id, "misses", 7).unwrap();
    let got = client.get_counter_items(&cache_id).unwrap();
    let map: std::collections::HashMap<_, _> = got.items.iter().map(|i| (i.key.clone(), i.value)).collect();
    assert_eq!(map.get("hits"), Some(&100));
    assert_eq!(map.get("misses"), Some(&7));

    client.clear_counter_cache(&cache_id).unwrap();
    client.delete_counter_cache(&cache_id).unwrap();
}

#[test]
fn bidi_get_stats() {
    let Some(url) = ws_url() else {
        println!("Skipping bidi_get_stats: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-stats");

    client.create_cache(&cache_id).unwrap();
    client.put_item(&cache_id, "k", "v").unwrap();
    let stats = client.get_stats().unwrap();
    // The freshly-populated cache should appear in the per-cache stats.
    assert!(stats.iter().any(|s| s.cache_id == cache_id), "stats did not include {cache_id}");
    client.delete_cache(&cache_id).unwrap();
}

#[test]
fn bidi_cancel_item_removal() {
    let Some(url) = ws_url() else {
        println!("Skipping bidi_cancel_item_removal: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-cancel");

    client.create_cache(&cache_id).unwrap();
    client.put_timed_item(&cache_id, "keep", "val", 2000).unwrap();
    assert_eq!(client.cancel_item_removal(&cache_id, "keep").unwrap().key, "keep");
    thread::sleep(Duration::from_secs(3));
    assert_eq!(client.get_item(&cache_id, "keep").unwrap().value, "val");
    client.delete_cache(&cache_id).unwrap();
}

#[test]
fn bidi_subscription_receives_add_item() {
    let Some(url) = ws_url() else {
        println!("Skipping bidi_subscription_receives_add_item: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-sub");
    client.create_cache(&cache_id).unwrap();

    let got: Arc<Mutex<Vec<(String, Option<String>)>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = got.clone();
    let sub = client
        .subscribe(&cache_id, move |ev| {
            if ev.event_type == "ADD_ITEM" && ev.item_key.as_deref() == Some("sk") {
                sink.lock().unwrap().push((ev.event_type.clone(), ev.item_value.clone()));
            }
        })
        .expect("subscribe");

    client.put_item(&cache_id, "sk", "sv").unwrap();

    // Poll with timeout for the streamed update.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if !got.lock().unwrap().is_empty() {
            break;
        }
        if Instant::now() >= deadline {
            panic!("did not receive ADD_ITEM stream update within timeout");
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(got.lock().unwrap()[0].1.as_deref(), Some("sv"));

    sub.close().unwrap();
    client.delete_cache(&cache_id).unwrap();
}
