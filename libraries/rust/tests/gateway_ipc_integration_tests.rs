//! End-to-end tests for the Aeron gateway transport over IPC (`TransportMedia::Ipc`) against a real
//! gateway.
//!
//! Enabled only when `AERON_GATEWAY_IPC_IT=true` is set, so they do not run in the default `cargo
//! test`. Unlike `gateway_integration_tests.rs` (UDP, own embedded driver), IPC requires this process
//! to share the gateway server's media driver, so the directory must be the *same* one the server is
//! using, from `AERON_DIR` (default `"aeron"`, matching the server's own default resolution). Enable
//! the gateway on the backend with `AERON_GATEWAY_ENABLED=true` and `GATEWAY_TRANSPORT_MEDIA=ipc`.
//!
//! The wire encoding/decoding is identical to the UDP transport (same SBE frames, same commands) —
//! this file exists to prove the IPC channel wiring itself, so it covers a representative subset of
//! operations (single command/response, batched-response accumulation, streaming, bulk) rather than
//! duplicating every case in `gateway_integration_tests.rs`. Run with `--test-threads=1`.

use aeron_cache_embedded_client::{
    AeronGatewayClient, BulkCacheOpsRequest, BulkOperationType, CacheOperationRequest,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn it_enabled() -> bool {
    std::env::var("AERON_GATEWAY_IPC_IT").map(|v| v == "true").unwrap_or(false)
}

fn aeron_dir() -> String {
    std::env::var("AERON_DIR").unwrap_or_else(|_| "aeron".to_string())
}

/// A connected IPC client, or `None` if these integration tests are disabled. No embedded driver is
/// launched — IPC connects to the driver already running at `aeron_dir()`, which must be the same one
/// the gateway server is using.
fn connect() -> Option<AeronGatewayClient> {
    if !it_enabled() {
        eprintln!("skipping: set AERON_GATEWAY_IPC_IT=true to run gateway IPC integration tests");
        return None;
    }
    let client = AeronGatewayClient::connect_ipc(&aeron_dir()).expect("failed to connect over IPC");
    assert!(client.await_connected(Duration::from_secs(10)), "gateway did not connect over IPC");
    Some(client)
}

fn unique(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix}-{nanos:x}")
}

// ------------------------------------------------------------------ multi-batch reassembly helpers
//
// Same wire framing as UDP: responses stream as one or more batch frames terminated by
// `end_of_batch=true`, accumulated internally by the client. The Rust gateway client exposes no
// per-batch callback (unlike Java's GatewayClientListener), so these are REASSEMBLY-COMPLETENESS tests
// — populating enough large entries forces a server-side split, and asserting all N come back intact
// proves reassembly over the IPC channel. This file carries a representative subset (entries + timers)
// matching its "representative subset" philosophy; the full set lives in gateway_integration_tests.rs.

/// Entry count chosen to overflow a single batch frame; mirrors the Java batch test's 1000.
const BATCH_ENTRY_COUNT: usize = 1_000;
/// Per-bulk chunk size kept well under the command ring buffer's max message length.
const BATCH_CHUNK: usize = 200;

/// Value long enough that `BATCH_ENTRY_COUNT` entries comfortably exceed one batch frame (300 chars).
fn big_value() -> String {
    "v".repeat(300)
}

/// Inserts `count` entries (keys `k0..k{count-1}`, each with [`big_value`]) via chunked bulk ADD_ITEM.
/// A positive `ttl` schedules a pending TTL removal timer per entry (used by the timers test).
fn populate_entries(client: &AeronGatewayClient, cache: &str, count: usize, ttl: Option<i64>) {
    let value = big_value();
    let mut start = 0;
    while start < count {
        let end = (start + BATCH_CHUNK).min(count);
        let operations = (start..end)
            .map(|i| CacheOperationRequest {
                operation_type: BulkOperationType::AddItem,
                request_id: format!("add-{i}"),
                cache_id: cache.to_string(),
                key: Some(format!("k{i}")),
                value: Some(value.clone()),
                ttl,
                counter_value: None,
            })
            .collect();
        let req = BulkCacheOpsRequest { request_id: unique("populate"), operations };
        let resp = client.bulk_ops(&req).unwrap();
        assert_eq!(resp.operation_responses.len(), end - start, "populate chunk should ack every op");
        start = end;
    }
}

#[test]
fn cache_lifecycle() {
    let Some(client) = connect() else { return };
    let cache = unique("rs-ipc-it-cache");

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
fn counter_lifecycle() {
    let Some(client) = connect() else { return };
    let cache = unique("rs-ipc-it-counter");

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
    let Some(client) = connect() else { return };
    let cache = unique("rs-ipc-it-stream");
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
fn get_timers_over_gateway() {
    let Some(client) = connect() else { return };
    let cache = unique("rs-ipc-it-timers");
    client.create_cache(&cache).unwrap();
    // A timed entry schedules a pending TTL removal timer; get_timers accumulates one or more
    // batches, exercising batched-response reassembly over IPC.
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
fn bulk_ops_over_gateway() {
    let Some(client) = connect() else { return };
    let cache = unique("rs-ipc-it-bulk");

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
fn get_cache_items_reassembles_across_multiple_batches() {
    // Reassembly-completeness over IPC (no per-batch seam — see the module comment above).
    let Some(client) = connect() else { return };
    let cache = unique("rs-ipc-it-batch-entries");
    client.create_cache(&cache).unwrap();
    populate_entries(&client, &cache, BATCH_ENTRY_COUNT, None);

    let resp = client.get_cache_items(&cache).unwrap();
    assert_eq!(
        resp.items.len(),
        BATCH_ENTRY_COUNT,
        "every entry should be reassembled from the batched response over IPC"
    );
    let value = big_value();
    let map: std::collections::HashMap<_, _> =
        resp.items.iter().map(|i| (i.key.clone(), i.value.clone())).collect();
    for i in 0..BATCH_ENTRY_COUNT {
        assert_eq!(map.get(&format!("k{i}")), Some(&value), "missing/incorrect value for k{i}");
    }

    client.delete_cache(&cache).unwrap();
}

#[test]
fn get_timers_reassembles_across_multiple_batches() {
    // Reassembly-completeness over IPC (no per-batch seam — see the module comment above).
    let Some(client) = connect() else { return };
    let cache = unique("rs-ipc-it-batch-timers");
    client.create_cache(&cache).unwrap();
    // A large TTL keeps every scheduled removal timer pending so all N are returned by get_timers.
    populate_entries(&client, &cache, BATCH_ENTRY_COUNT, Some(600_000));

    let timers = client.get_timers().unwrap();
    let mut ours: std::collections::HashSet<String> = std::collections::HashSet::new();
    for t in timers.iter().filter(|t| t.cache_id == cache) {
        assert_eq!(t.timer_type, "CACHE");
        assert!(t.deadline > 0, "timer deadline should be a positive epoch millis");
        ours.insert(t.key.clone());
    }
    assert_eq!(
        ours.len(),
        BATCH_ENTRY_COUNT,
        "every pending timer for the cache should be reassembled from the batched response over IPC"
    );

    client.delete_cache(&cache).unwrap();
}

#[test]
fn cancel_item_removal_over_ipc() {
    let Some(client) = connect() else { return };
    let cache = unique("rs-ipc-it-cancel");
    client.create_cache(&cache).unwrap();
    client.put_timed_item(&cache, "keep", "val", 2000).unwrap();

    let resp = client.cancel_item_removal(&cache, "keep").unwrap();
    assert_eq!(resp.key, "keep");

    std::thread::sleep(Duration::from_secs(3));
    assert_eq!(
        client.get_item(&cache, "keep").unwrap().value,
        "val",
        "item should survive past its TTL after cancelling removal over IPC"
    );

    client.delete_cache(&cache).unwrap();
}
