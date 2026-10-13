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

// ------------------------------------------------------------------ multi-batch reassembly helpers
//
// The bidi transport streams getEntries/getTimers responses as one or more JSON batch frames
// terminated by `endOfBatch=true`; the client accumulates them internally keyed on correlationId
// (see `src/bidi.rs`) and exposes only the fully reassembled high-level result — there is no public
// per-batch callback where the flag is observable. So these are REASSEMBLY-COMPLETENESS tests:
// populating enough large entries forces the server to split the response across multiple batches, and
// asserting all N come back intact proves reassembly. The existing single-item bidi tests fit in one
// batch and never exercise this.

// 300 entries force the server to split the getCacheItems/getTimers response across multiple batch
// frames while keeping the write load light enough for a cold CI RAFT cluster (populating 1000
// entries timed out on CI).
const BATCH_ENTRY_COUNT: usize = 300;
// Small so each populate bulk commits well within the bidi request timeout on a slow CI cluster, and
// well under the bidi WebSocket ~64KB max text-message limit (else the server closes with 1009).
const BATCH_CHUNK: usize = 20;

fn big_value() -> String {
    "v".repeat(300)
}

/// Inserts `count` entries (keys `k0..k{count-1}`) into `cache_id` via chunked bulk ADD_ITEM. A
/// positive `ttl` schedules a pending TTL removal timer per entry (used by the timers test).
fn populate_entries(client: &AeronBidiClient, cache_id: &str, count: usize, ttl: Option<i64>) {
    use aeron_cache_embedded_client::{BulkCacheOpsRequest, BulkOperationType, CacheOperationRequest};
    let value = big_value();
    let mut start = 0;
    while start < count {
        let end = (start + BATCH_CHUNK).min(count);
        let operations = (start..end)
            .map(|i| CacheOperationRequest {
                operation_type: BulkOperationType::AddItem,
                request_id: format!("add-{i}"),
                cache_id: cache_id.to_string(),
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
fn bidi_get_caches() {
    let Some(url) = ws_url() else {
        println!("Skipping bidi_get_caches: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-caches");

    client.create_cache(&cache_id).unwrap();
    client.put_item(&cache_id, "k", "v").unwrap();
    let caches = client.get_caches().unwrap();
    let details = caches.iter().find(|c| c.cache_id == cache_id);
    assert!(details.is_some(), "get_caches did not include {cache_id}");
    assert!(details.unwrap().item_count >= 1, "item_count should reflect the stored entry");
    client.delete_cache(&cache_id).unwrap();
}

#[test]
fn bidi_get_counter_caches() {
    let Some(url) = ws_url() else {
        println!("Skipping bidi_get_counter_caches: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-counter-caches");

    client.create_counter_cache(&cache_id).unwrap();
    client.put_counter(&cache_id, "hits", 1).unwrap();
    let caches = client.get_counter_caches().unwrap();
    assert!(
        caches.iter().any(|c| c.cache_id == cache_id),
        "get_counter_caches did not include {cache_id}"
    );
    client.clear_counter_cache(&cache_id).unwrap();
    client.delete_counter_cache(&cache_id).unwrap();
}

#[test]
fn bidi_get_timers() {
    let Some(url) = ws_url() else {
        println!("Skipping bidi_get_timers: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-timers");

    client.create_cache(&cache_id).unwrap();
    // A timed entry schedules a pending TTL removal timer.
    client.put_timed_item(&cache_id, "ttl-key", "v", 600_000).unwrap();

    let timers = client.get_timers().unwrap();
    let timer = timers
        .iter()
        .find(|t| t.cache_id == cache_id && t.key == "ttl-key")
        .expect("a pending timer for our cache/key");
    assert_eq!(timer.timer_type, "CACHE");
    assert!(timer.deadline > 0, "timer deadline should be a positive epoch millis");
    client.delete_cache(&cache_id).unwrap();
}

#[test]
fn bidi_bulk_ops() {
    use aeron_cache_embedded_client::{BulkCacheOpsRequest, BulkOperationType, CacheOperationRequest};

    let Some(url) = ws_url() else {
        println!("Skipping bidi_bulk_ops: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-bulk");
    client.create_cache(&cache_id).unwrap();

    let op = |op_type, request_id: &str, key: &str, value: Option<&str>| CacheOperationRequest {
        operation_type: op_type,
        request_id: request_id.to_string(),
        cache_id: cache_id.clone(),
        key: Some(key.to_string()),
        value: value.map(|v| v.to_string()),
        ttl: None,
        counter_value: None,
    };
    let req = BulkCacheOpsRequest {
        request_id: unique("req"),
        operations: vec![
            op(BulkOperationType::AddItem, "op-1", "bk1", Some("bv1")),
            op(BulkOperationType::AddItem, "op-2", "bk2", Some("bv2")),
            op(BulkOperationType::GetItem, "op-3", "bk1", None),
        ],
    };

    let resp = client.bulk_ops(&req).unwrap();
    assert_eq!(resp.request_id, req.request_id);
    assert_eq!(resp.operation_responses.len(), 3);
    let get_result = resp
        .operation_responses
        .iter()
        .find(|r| r.request_id == "op-3")
        .expect("a result for op-3");
    assert_eq!(get_result.value.as_deref(), Some("bv1"));

    assert_eq!(client.get_item(&cache_id, "bk2").unwrap().value, "bv2");
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
fn bidi_get_cache_items_reassembles_across_multiple_batches() {
    // Reassembly-completeness (the bidi client hides batches behind its high-level API — see the
    // module comment above).
    let Some(url) = ws_url() else {
        println!("Skipping bidi_get_cache_items_reassembles_across_multiple_batches: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-batch-entries");
    client.create_cache(&cache_id).unwrap();
    populate_entries(&client, &cache_id, BATCH_ENTRY_COUNT, None);

    let resp = client.get_cache_items(&cache_id).unwrap();
    assert_eq!(
        resp.items.len(),
        BATCH_ENTRY_COUNT,
        "every entry should be reassembled from the batched response"
    );
    let value = big_value();
    let map: std::collections::HashMap<_, _> =
        resp.items.iter().map(|i| (i.key.clone(), i.value.clone())).collect();
    for i in 0..BATCH_ENTRY_COUNT {
        assert_eq!(map.get(&format!("k{i}")), Some(&value), "missing/incorrect value for k{i}");
    }

    client.delete_cache(&cache_id).unwrap();
}

#[test]
fn bidi_get_timers_reassembles_across_multiple_batches() {
    // Reassembly-completeness (the bidi client hides batches behind its high-level API — see the
    // module comment above).
    let Some(url) = ws_url() else {
        println!("Skipping bidi_get_timers_reassembles_across_multiple_batches: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-batch-timers");
    client.create_cache(&cache_id).unwrap();
    // A large TTL keeps every scheduled removal timer pending so all N are returned by get_timers.
    populate_entries(&client, &cache_id, BATCH_ENTRY_COUNT, Some(600_000));

    let timers = client.get_timers().unwrap();
    let mut ours: std::collections::HashSet<String> = std::collections::HashSet::new();
    for t in timers.iter().filter(|t| t.cache_id == cache_id) {
        assert_eq!(t.timer_type, "CACHE");
        assert!(t.deadline > 0, "timer deadline should be a positive epoch millis");
        ours.insert(t.key.clone());
    }
    assert_eq!(
        ours.len(),
        BATCH_ENTRY_COUNT,
        "every pending timer for the cache should be reassembled from the batched response"
    );

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

#[test]
fn bidi_subscription_key_filter() {
    let Some(url) = ws_url() else {
        println!("Skipping bidi_subscription_key_filter: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-sub-keys");
    client.create_cache(&cache_id).unwrap();

    // Subscribe filtered to only "key1" (no snapshot, full mode).
    let keys: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = keys.clone();
    let sub = client
        .subscribe_with(&cache_id, false, Some("key1"), None, move |ev| {
            if ev.event_type == "ADD_ITEM" {
                if let Some(k) = ev.item_key.clone() {
                    sink.lock().unwrap().push(k);
                }
            }
        })
        .expect("subscribe with key filter");

    client.put_item(&cache_id, "key1", "v1").unwrap();
    client.put_item(&cache_id, "key2", "v2").unwrap();

    // Poll until the subscribed key arrives, then confirm the filtered-out key never does.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if keys.lock().unwrap().iter().any(|k| k == "key1") {
            break;
        }
        if Instant::now() >= deadline {
            panic!("did not receive ADD_ITEM for key1 within timeout");
        }
        thread::sleep(Duration::from_millis(50));
    }
    // Give any stray (incorrectly unfiltered) key2 event a chance to show up.
    thread::sleep(Duration::from_millis(500));
    let got = keys.lock().unwrap();
    assert!(got.iter().any(|k| k == "key1"), "should receive key1 event");
    assert!(!got.iter().any(|k| k == "key2"), "should NOT receive key2 event (filtered out)");
    drop(got);

    sub.close().unwrap();
    client.delete_cache(&cache_id).unwrap();
}

#[test]
fn bidi_subscription_patch_mode_receives_patch_item() {
    let Some(url) = ws_url() else {
        println!("Skipping bidi_subscription_patch_mode_receives_patch_item: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-sub-patch");
    client.create_cache(&cache_id).unwrap();
    // Seed the initial object so patching it yields a PATCH_ITEM delta.
    client.put_item(&cache_id, "doc", r#"{"a":1}"#).unwrap();

    // Subscribe in patch mode: changed fields arrive as PATCH_ITEM deltas rather than full ADD_ITEMs.
    let got: Arc<Mutex<Vec<(String, Option<String>)>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = got.clone();
    let sub = client
        .subscribe_with(&cache_id, false, None, Some("patch"), move |ev| {
            if ev.event_type == "PATCH_ITEM" && ev.item_key.as_deref() == Some("doc") {
                sink.lock().unwrap().push((ev.event_type.clone(), ev.item_value.clone()));
            }
        })
        .expect("subscribe in patch mode");

    client.patch_item(&cache_id, "doc", r#"{"b":2}"#).unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if !got.lock().unwrap().is_empty() {
            break;
        }
        if Instant::now() >= deadline {
            panic!("did not receive PATCH_ITEM stream update within timeout");
        }
        thread::sleep(Duration::from_millis(50));
    }
    // The delta carries only the changed field.
    let delta = got.lock().unwrap()[0].1.clone().expect("PATCH_ITEM should carry a delta value");
    assert!(delta.contains("\"b\""), "patch delta should contain the changed field, was {delta}");

    sub.close().unwrap();
    client.delete_cache(&cache_id).unwrap();
}

#[test]
fn bidi_counter_subscription_receives_update() {
    let Some(url) = ws_url() else {
        println!("Skipping bidi_counter_subscription_receives_update: AERON_CACHE_BASE_URL not set");
        return;
    };
    let client = AeronBidiClient::connect(&url).expect("connect");
    let cache_id = unique("bidi-rs-sub-counter");
    client.create_counter_cache(&cache_id).unwrap();

    let got: Arc<Mutex<Vec<(String, Option<i64>)>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = got.clone();
    let sub = client
        .subscribe_counter(&cache_id, move |ev| {
            if ev.item_key.as_deref() == Some("ck") {
                sink.lock().unwrap().push((ev.event_type.clone(), ev.item_value));
            }
        })
        .expect("subscribe counter");

    client.put_counter(&cache_id, "ck", 7).unwrap();
    client.increment_counter(&cache_id, "ck", 3).unwrap();

    // Poll for the streamed counter update reflecting the latest value.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if got.lock().unwrap().iter().any(|(_, v)| *v == Some(10)) {
            break;
        }
        if Instant::now() >= deadline {
            panic!("did not receive counter stream update within timeout");
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(
        got.lock().unwrap().iter().any(|(_, v)| *v == Some(10)),
        "should receive a counter update with the incremented value"
    );

    sub.close().unwrap();
    client.delete_counter_cache(&cache_id).unwrap();
}
