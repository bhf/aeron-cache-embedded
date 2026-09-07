//! End-to-end tests for the Aeron gateway transport against a real gateway.
//!
//! Enabled only when `AERON_GATEWAY_IT=true` is set, so they do not run in the default `cargo test`.
//! The gateway must be reachable at `AERON_GATEWAY_HOST` (default `127.0.0.1`) on the default
//! request/response ports. Enable the gateway on the backend with `AERON_TRANSPORT_GATEWAY_ENABLED=true`
//! and pin its endpoints to loopback with `GATEWAY_REQUEST_ENDPOINT=127.0.0.1:7075` and
//! `GATEWAY_RESPONSE_CONTROL_ENDPOINT=127.0.0.1:7076`.
//!
//! Each test launches its own embedded media driver and talks to the gateway over UDP.

use aeron_cache_embedded_client::AeronGatewayClient;
use rusteron_media_driver::testing::EmbeddedDriver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn it_enabled() -> bool {
    std::env::var("AERON_GATEWAY_IT").map(|v| v == "true").unwrap_or(false)
}

fn host() -> String {
    std::env::var("AERON_GATEWAY_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

/// Launch an embedded driver and a connected client, or `None` if integration tests are disabled.
fn connect() -> Option<(EmbeddedDriver, AeronGatewayClient)> {
    if !it_enabled() {
        eprintln!("skipping: set AERON_GATEWAY_IT=true to run gateway integration tests");
        return None;
    }
    let driver = EmbeddedDriver::launch().expect("failed to launch embedded media driver");
    let client = AeronGatewayClient::connect(driver.dir(), &host()).expect("failed to connect");
    assert!(client.await_connected(Duration::from_secs(10)), "gateway did not connect");
    Some((driver, client))
}

fn unique(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix}-{nanos:x}")
}

#[test]
fn cache_lifecycle() {
    let Some((_driver, client)) = connect() else { return };
    let cache = unique("rs-it-cache");

    let created = client.create_cache(&cache).unwrap();
    assert_eq!(created.cache_id, cache);

    let put = client.put_item(&cache, "k1", "v1").unwrap();
    assert_eq!(put.key, "k1");

    let got = client.get_item(&cache, "k1").unwrap();
    assert_eq!(got.value, "v1");

    let removed = client.delete_item(&cache, "k1").unwrap();
    assert_eq!(removed.key, "k1");

    let deleted = client.delete_cache(&cache).unwrap();
    assert_eq!(deleted.cache_id, cache);
}

#[test]
fn get_cache_items_and_clear() {
    let Some((_driver, client)) = connect() else { return };
    let cache = unique("rs-it-entries");
    client.create_cache(&cache).unwrap();
    client.put_item(&cache, "a", "1").unwrap();
    client.put_item(&cache, "b", "2").unwrap();

    let all = client.get_cache_items(&cache).unwrap();
    assert_eq!(all.items.len(), 2);

    client.clear_cache(&cache).unwrap();
    let after = client.get_cache_items(&cache).unwrap();
    assert!(after.items.is_empty(), "cache should be empty after clear");

    client.delete_cache(&cache).unwrap();
}

#[test]
fn get_stats() {
    let Some((_driver, client)) = connect() else { return };
    let cache = unique("rs-it-stats");
    client.create_cache(&cache).unwrap();
    client.put_item(&cache, "s1", "v1").unwrap();
    client.put_item(&cache, "s2", "v2").unwrap();

    let stats = client.get_stats().unwrap();
    let stat = stats.iter().find(|s| s.cache_id == cache).expect("stats for our cache");
    assert_eq!(stat.size, 2);

    client.delete_cache(&cache).unwrap();
}

#[test]
fn counter_lifecycle() {
    let Some((_driver, client)) = connect() else { return };
    let cache = unique("rs-it-counter");

    client.create_counter_cache(&cache).unwrap();
    client.put_counter(&cache, "hits", 10).unwrap();

    assert_eq!(client.increment_counter(&cache, "hits", 5).unwrap().value, 15);
    assert_eq!(client.decrement_counter(&cache, "hits", 3).unwrap().value, 12);
    assert_eq!(client.set_counter(&cache, "hits", 100).unwrap().value, 100);
    assert_eq!(client.get_counter(&cache, "hits").unwrap().value, 100);

    client.delete_counter(&cache, "hits").unwrap();
    client.delete_counter_cache(&cache).unwrap();
}

#[test]
fn streaming_updates() {
    let Some((_driver, client)) = connect() else { return };
    let cache = unique("rs-it-stream");
    client.create_cache(&cache).unwrap();

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let sub = client.subscribe(&cache, move |e| sink.lock().unwrap().push(e)).unwrap();

    std::thread::sleep(Duration::from_millis(500));
    client.put_item(&cache, "sk", "sv").unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    while events.lock().unwrap().is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    let found = events.lock().unwrap().iter().any(|e| e.item_key.as_deref() == Some("sk"));
    assert!(found, "expected a streamed update for key sk");

    drop(sub);
    client.delete_cache(&cache).unwrap();
}

#[test]
fn counter_streaming_updates() {
    let Some((_driver, client)) = connect() else { return };
    let cache = unique("rs-it-cstream");
    client.create_counter_cache(&cache).unwrap();

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let sub = client.subscribe_counter(&cache, move |e| sink.lock().unwrap().push(e)).unwrap();

    std::thread::sleep(Duration::from_millis(500));
    client.put_counter(&cache, "ck", 7).unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    while events.lock().unwrap().is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    let found = events.lock().unwrap().iter().any(|e| e.item_key.as_deref() == Some("ck"));
    assert!(found, "expected a streamed counter update for key ck");

    drop(sub);
    client.delete_counter_cache(&cache).unwrap();
}
