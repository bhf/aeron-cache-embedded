//! Demonstrates the bidirectional WebSocket transport for Aeron Cache using
//! [`AeronBidiClient`] — a single `/api/ws/v1/bidi` WebSocket connection that carries the full
//! cache + counter command surface plus dynamic subscribe/unsubscribe.
//!
//! Override the WebSocket URL with the first CLI argument (default `ws://localhost:7071`).

use aeron_cache_embedded_client::{
    AeronBidiClient, BulkCacheOpsRequest, BulkOperationType, CacheOperationRequest,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ws_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://localhost:7071".to_string());
    println!("Starting BIDI Sample against {ws_url}");

    let client = AeronBidiClient::connect(&ws_url)?;
    println!("Connected over bidirectional WebSocket.");

    // --- Cache operations ---
    let cache_id = "bidi-sample-cache";
    println!("Created cache: {}", client.create_cache(cache_id)?.cache_id);
    println!(
        "Put bidi-key -> bidi-value: {}",
        client.put_item(cache_id, "bidi-key", "bidi-value")?.operation_status
    );
    println!("Get bidi-key -> {}", client.get_item(cache_id, "bidi-key")?.value);

    // --- Patch (JSON merge) ---
    println!("\n--- Patch ---");
    client.put_item(cache_id, "doc", "{\"a\":1}")?;
    println!("Put doc -> {}", client.get_item(cache_id, "doc")?.value);
    println!(
        "Patch doc with {{\"b\":2}}: {}",
        client.patch_item(cache_id, "doc", "{\"b\":2}")?.operation_status
    );
    println!("Merged doc -> {}", client.get_item(cache_id, "doc")?.value);

    // --- Counters ---
    println!("\n--- Counters ---");
    let counter_cache = "bidi-sample-counters";
    println!(
        "Created counter cache: {}",
        client.create_counter_cache(counter_cache)?.cache_id
    );
    println!("Put hits = 10 -> {}", client.put_counter(counter_cache, "hits", 10)?.operation_status);
    println!("increment hits +5 -> {}", client.increment_counter(counter_cache, "hits", 5)?.value);
    println!("Get hits -> {}", client.get_counter(counter_cache, "hits")?.value);

    // --- Bulk read + stats ---
    println!("\n--- Cache items + stats ---");
    let items = client.get_cache_items(cache_id)?;
    println!("Items in {}:", items.cache_id);
    for item in &items.items {
        println!("  {} = {}", item.key, item.value);
    }
    println!("Stats:");
    for entry in client.get_stats()? {
        println!(
            "  {} size={} added={} removed={} cleared={}",
            entry.cache_id, entry.size, entry.added_count, entry.removed_count, entry.cleared_count
        );
    }

    // --- Bulk operations ---
    // A single `bulk` frame carries a batch of operations (regular-cache and counter ops may be
    // mixed); the server streams back per-operation results, each echoing its own requestId.
    println!("\n--- Bulk operations ---");
    let bulk = BulkCacheOpsRequest {
        request_id: "bidi-bulk-1".to_string(),
        operations: vec![
            CacheOperationRequest {
                operation_type: BulkOperationType::AddItem,
                request_id: "op-1".to_string(),
                cache_id: cache_id.to_string(),
                key: Some("bk1".to_string()),
                value: Some("bv1".to_string()),
                ttl: None,
                counter_value: None,
            },
            CacheOperationRequest {
                operation_type: BulkOperationType::AddItem,
                request_id: "op-2".to_string(),
                cache_id: cache_id.to_string(),
                key: Some("bk2".to_string()),
                value: Some("bv2".to_string()),
                ttl: None,
                counter_value: None,
            },
            CacheOperationRequest {
                operation_type: BulkOperationType::GetItem,
                request_id: "op-3".to_string(),
                cache_id: cache_id.to_string(),
                key: Some("bk1".to_string()),
                value: None,
                ttl: None,
                counter_value: None,
            },
        ],
    };
    for op in client.bulk_ops(&bulk)?.operation_responses {
        match op.value.filter(|v| !v.is_empty()) {
            Some(value) => println!("  {} -> {} ({})", op.request_id, op.status, value),
            None => println!("  {} -> {}", op.request_id, op.status),
        }
    }

    // --- Timers ---
    // A timed entry schedules a pending TTL removal timer; getTimers lists all pending timers
    // (cache + counter), each tagged with its type.
    println!("\n--- Timers ---");
    client.put_timed_item(cache_id, "expiring", "gone-soon", 600_000)?;
    for timer in client.get_timers()? {
        println!(
            "  [{}] {}/{} fires at {}",
            timer.timer_type, timer.cache_id, timer.key, timer.deadline
        );
    }

    // --- Live subscription ---
    println!("\n--- Streaming subscription ---");
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let subscription = client.subscribe(cache_id, move |event| {
        println!(
            "  [update] {} {}={:?}",
            event.event_type,
            event.item_key.clone().unwrap_or_default(),
            event.item_value
        );
        sink.lock().unwrap().push(event);
    })?;
    std::thread::sleep(Duration::from_millis(500));
    client.put_item(cache_id, "streamed-key", "streamed-value")?;
    std::thread::sleep(Duration::from_millis(1000));
    println!("Received {} streamed event(s).", events.lock().unwrap().len());
    subscription.close()?;

    // --- Cleanup ---
    println!("\n--- Cleanup ---");
    println!("Delete cache: {}", client.delete_cache(cache_id)?.operation_status);
    println!("Delete counter cache: {}", client.delete_counter_cache(counter_cache)?.operation_status);
    println!("Done.");
    Ok(())
}
