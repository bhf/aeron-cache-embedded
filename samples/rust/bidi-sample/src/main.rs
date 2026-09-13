//! Demonstrates the bidirectional WebSocket transport for Aeron Cache using
//! [`AeronBidiClient`] — a single `/api/ws/v1/bidi` WebSocket connection that carries the full
//! cache + counter command surface plus dynamic subscribe/unsubscribe.
//!
//! Override the WebSocket URL with the first CLI argument (default `ws://localhost:7071`).

use aeron_cache_embedded_client::AeronBidiClient;
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
