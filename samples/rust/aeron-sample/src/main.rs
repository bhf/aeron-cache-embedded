//! Demonstrates the Aeron gateway transport for Aeron Cache — the low-latency, bidirectional
//! alternative to the HTTP+WS client.
//!
//! Launches an embedded media driver and talks to a gateway over UDP. Enable the gateway on the
//! backend with `AERON_TRANSPORT_GATEWAY_ENABLED=true`. Override the host with the first CLI argument
//! (default `127.0.0.1`).

use aeron_cache_embedded_client::AeronGatewayClient;
use rusteron_media_driver::testing::EmbeddedDriver;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1".to_string());
    println!("Starting Aeron Sample against gateway {host}");

    // Embedded media driver so the sample is self-contained; the gateway is reached over UDP.
    let driver = EmbeddedDriver::launch()?;
    let client = AeronGatewayClient::connect(driver.dir(), &host)?;

    if !client.await_connected(Duration::from_secs(10)) {
        eprintln!("Could not connect to the gateway — is it enabled and reachable?");
        return Ok(());
    }
    println!("Connected to gateway.");

    // --- Cache operations ---
    let cache_id = "aeron-sample-cache";
    println!("Created cache: {}", client.create_cache(cache_id)?.cache_id);
    println!("Put aeron-key -> aeron-value: {}", client.put_item(cache_id, "aeron-key", "aeron-value")?.operation_status);
    println!("Get aeron-key -> {}", client.get_item(cache_id, "aeron-key")?.value);

    // --- Streaming subscription ---
    println!("\n--- Streaming ---");
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let subscription = client.subscribe(cache_id, move |event| {
        println!("  [update] {} {}={:?}", event.event_type, event.item_key.clone().unwrap_or_default(), event.item_value);
        sink.lock().unwrap().push(event);
    })?;
    std::thread::sleep(Duration::from_millis(500));
    client.put_item(cache_id, "streamed-key", "streamed-value")?;
    std::thread::sleep(Duration::from_millis(1000));
    drop(subscription);

    // --- Counter operations ---
    println!("\n--- Counters ---");
    let counter_cache = "aeron-sample-counters";
    println!("Created counter cache: {}", client.create_counter_cache(counter_cache)?.cache_id);
    client.put_counter(counter_cache, "hits", 10)?;
    println!("increment hits +5 -> {}", client.increment_counter(counter_cache, "hits", 5)?.value);
    println!("decrement hits -3 -> {}", client.decrement_counter(counter_cache, "hits", 3)?.value);
    println!("set hits = 100 -> {}", client.set_counter(counter_cache, "hits", 100)?.value);

    client.delete_cache(cache_id)?;
    client.delete_counter_cache(counter_cache)?;
    println!("Done.");
    Ok(())
}
