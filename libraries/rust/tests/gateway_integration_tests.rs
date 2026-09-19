//! End-to-end tests for the Aeron gateway transport against a real gateway.
//!
//! Enabled only when `AERON_GATEWAY_IT=true` is set, so they do not run in the default `cargo test`.
//! The gateway must be reachable at `AERON_GATEWAY_HOST` (default `127.0.0.1`) on the default
//! request/response ports. Enable the gateway on the backend with `AERON_GATEWAY_ENABLED=true`
//! and pin its endpoints to loopback with `GATEWAY_REQUEST_ENDPOINT=127.0.0.1:7075` and
//! `GATEWAY_RESPONSE_CONTROL_ENDPOINT=127.0.0.1:7076`.
//!
//! Each test launches its own embedded media driver and talks to the gateway over UDP.

use aeron_cache_embedded_client::{
    AeronGatewayClient, BulkCacheOpsRequest, BulkOperationType, CacheOperationRequest, CacheTransport,
};
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
fn get_timers_over_gateway() {
    let Some((_driver, client)) = connect() else { return };
    let cache = unique("rs-it-timers");
    client.create_cache(&cache).unwrap();
    // A timed entry schedules a pending TTL removal timer.
    client.put_timed_item(&cache, "ttl-key", "v", 600_000).unwrap();

    let timers = client.get_timers().unwrap();
    let timer = timers
        .iter()
        .find(|t| t.cache_id == cache && t.key == "ttl-key")
        .expect("a pending timer for our cache/key");
    assert_eq!(timer.timer_type, "CACHE");
    assert!(timer.deadline > 0, "timer deadline should be a positive epoch millis");

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
fn embedded_cache_mirrors_over_aeron() {
    let Some((_driver, client)) = connect() else { return };
    let cache_name = unique("rs-it-embedded");
    client.create_cache(&cache_name).unwrap();

    let embedded = client.embedded_cache(&cache_name);
    let sub = embedded.subscribe().unwrap();
    std::thread::sleep(Duration::from_millis(500));
    embedded.insert("ek", "ev").unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    while embedded.get_local("ek").is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(embedded.get_local("ek").as_deref(), Some("ev"));

    drop(sub);
    client.delete_cache(&cache_name).unwrap();
}

#[test]
fn embedded_counter_mirrors_over_aeron() {
    let Some((_driver, client)) = connect() else { return };
    let cache_name = unique("rs-it-embedded-counter");
    client.create_counter_cache(&cache_name).unwrap();

    let embedded = client.embedded_counter_cache(&cache_name);
    let sub = embedded.subscribe().unwrap();
    std::thread::sleep(Duration::from_millis(500));
    embedded.insert("ec", 42).unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    while embedded.get_local("ec").is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(embedded.get_local("ec"), Some(42));

    drop(sub);
    client.delete_counter_cache(&cache_name).unwrap();
}

#[test]
fn bulk_ops_over_gateway() {
    let Some((_driver, client)) = connect() else { return };
    let cache = unique("rs-it-bulk");

    let req = BulkCacheOpsRequest {
        request_id: unique("bulk-req"),
        operations: vec![
            CacheOperationRequest {
                operation_type: BulkOperationType::CreateCache,
                request_id: "op-create".to_string(),
                cache_id: cache.clone(),
                key: None,
                value: None,
                ttl: None,
                counter_value: None,
            },
            CacheOperationRequest {
                operation_type: BulkOperationType::AddItem,
                request_id: "op-add".to_string(),
                cache_id: cache.clone(),
                key: Some("bk".to_string()),
                value: Some("bv".to_string()),
                ttl: None,
                counter_value: None,
            },
            CacheOperationRequest {
                operation_type: BulkOperationType::GetItem,
                request_id: "op-get".to_string(),
                cache_id: cache.clone(),
                key: Some("bk".to_string()),
                value: None,
                ttl: None,
                counter_value: None,
            },
        ],
    };

    let resp = client.bulk_ops(&req).unwrap();
    assert_eq!(resp.operation_responses.len(), 3, "expected one response per operation");

    let get = resp
        .operation_responses
        .iter()
        .find(|r| r.request_id == "op-get")
        .expect("response for op-get");
    assert_eq!(get.value.as_deref(), Some("bv"));

    client.delete_cache(&cache).unwrap();
}

#[test]
fn keyed_subscription() {
    // Exercises the new key-filtered subscription (`key` field on GatewaySubscribe) in FULL mode.
    // Patch mode (SubscriptionMode::PATCH / PATCH_ITEM) is NOT exercisable over the gateway — there
    // is no patch command in the gateway wire protocol (deep-merge patch is HTTP-only), so a put
    // never produces a PATCH_ITEM. Patch-mode *encoding* is covered by the SBE round-trip unit tests.
    let Some((_driver, client)) = connect() else { return };
    let cache = unique("rs-it-keyed");
    client.create_cache(&cache).unwrap();

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let sub = client
        .subscribe_with(&cache, false, None, Some("pk"), move |e| sink.lock().unwrap().push(e))
        .unwrap();

    std::thread::sleep(Duration::from_millis(500));
    client.put_item(&cache, "pk", "pv1").unwrap();
    client.put_item(&cache, "other", "ov1").unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    while !events.lock().unwrap().iter().any(|e| e.item_key.as_deref() == Some("pk"))
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(50));
    }
    let keys: Vec<Option<String>> = events.lock().unwrap().iter().map(|e| e.item_key.clone()).collect();
    eprintln!("keyed_subscription received keys: {:?}", keys);
    assert!(
        keys.iter().any(|k| k.as_deref() == Some("pk")),
        "expected a streamed update for the subscribed key pk"
    );
    assert!(
        !keys.iter().any(|k| k.as_deref() == Some("other")),
        "key filter should exclude 'other'"
    );

    drop(sub);
    client.delete_cache(&cache).unwrap();
}

#[test]
fn patch_mode_subscription() {
    // Real patch-mode subscription: patch an existing item over the gateway (PATCH_CACHE_ENTRY) and
    // observe the PATCH_ITEM delta stream. Requires the gateway's patch command (msgType 12).
    let Some((_driver, client)) = connect() else { return };
    let cache = unique("rs-it-patch");
    client.create_cache(&cache).unwrap();
    client.put_item(&cache, "pk", "{\"a\":1}").unwrap();

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let sub = client
        .subscribe_with(&cache, false, Some("patch"), Some("pk"), move |e| sink.lock().unwrap().push(e))
        .unwrap();

    std::thread::sleep(Duration::from_millis(500));
    client.patch_item(&cache, "pk", "{\"b\":2}").unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    while !events.lock().unwrap().iter().any(|e| e.event_type == "PATCH_ITEM")
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(50));
    }
    let found = events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e.event_type == "PATCH_ITEM" && e.item_key.as_deref() == Some("pk"));
    assert!(found, "expected a PATCH_ITEM event for key pk in patch mode");

    drop(sub);
    client.delete_cache(&cache).unwrap();
}

#[test]
fn cancel_item_removal_over_gateway() {
    let Some((_driver, client)) = connect() else { return };
    let cache = unique("rs-it-gw-cancel");
    client.create_cache(&cache).unwrap();
    client.put_timed_item(&cache, "keep", "val", 2000).unwrap();

    let resp = client.cancel_item_removal(&cache, "keep").unwrap();
    assert_eq!(resp.key, "keep");

    std::thread::sleep(Duration::from_secs(3));
    assert_eq!(client.get_item(&cache, "keep").unwrap().value, "val",
        "item should survive past its TTL after cancelling removal");

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
