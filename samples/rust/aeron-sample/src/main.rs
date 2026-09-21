//! Demonstrates the Aeron gateway transport for Aeron Cache — the low-latency, bidirectional
//! alternative to the HTTP+WS client.
//!
//! By default this launches an embedded media driver and talks to a gateway over UDP. Enable the
//! gateway on the backend with `AERON_GATEWAY_ENABLED=true`. Override the host with the first CLI
//! argument (default `127.0.0.1`).
//!
//! Set `AERON_GATEWAY_MEDIA=ipc` to instead connect over IPC. IPC has no network endpoints — this
//! process must run on the same host as the gateway server and share its media driver directory, so
//! no embedded driver is launched; `AERON_DIR` (matching what the backend was started with) selects
//! the shared driver instead. Start the backend with `GATEWAY_TRANSPORT_MEDIA=ipc AERON_DIR=<dir>
//! aeron-cache` so the client and server agree.

use aeron_cache_embedded_client::{AeronGatewayClient, CacheTransport, TransportMedia};
use rusteron_media_driver::testing::EmbeddedDriver;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let media = TransportMedia::parse(&std::env::var("AERON_GATEWAY_MEDIA").unwrap_or_default());

    // Keeps the embedded driver alive for the process lifetime in UDP mode; unused (and unlaunched)
    // for IPC, which shares the gateway server's own driver instead.
    let mut _embedded_driver: Option<EmbeddedDriver> = None;
    let client = if media == TransportMedia::Ipc {
        let aeron_dir = std::env::var("AERON_DIR").unwrap_or_else(|_| "aeron".to_string());
        println!("Starting Aeron Sample over IPC, shared media driver at {aeron_dir}");
        AeronGatewayClient::connect_ipc(&aeron_dir)?
    } else {
        let host = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1".to_string());
        println!("Starting Aeron Sample against gateway {host} over UDP");
        // Embedded media driver so the UDP sample is self-contained.
        let driver = EmbeddedDriver::launch()?;
        let client = AeronGatewayClient::connect(driver.dir(), &host)?;
        _embedded_driver = Some(driver);
        client
    };

    if !client.await_connected(Duration::from_secs(10)) {
        eprintln!(
            "Could not connect to the gateway — is it enabled and reachable, and (for IPC) \
             is AERON_DIR the same directory the gateway server is using?"
        );
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

    // --- Embedded cache over the gateway (transport-neutral: same API over HTTP+WS or Aeron) ---
    println!("\n--- Embedded cache ---");
    let embedded = client.embedded_cache("aeron-sample-embedded");
    client.create_cache("aeron-sample-embedded")?;
    let subscription = embedded.subscribe()?; // local mirror now updates in the background
    std::thread::sleep(Duration::from_millis(500));
    embedded.insert("user:1", "Ada")?;
    embedded.insert("user:2", "Alan")?;
    std::thread::sleep(Duration::from_millis(1000));
    println!("Local read user:1 -> {:?}", embedded.get_local("user:1"));
    println!("Local mirror snapshot -> {:?}", embedded.local_snapshot());
    drop(subscription);
    client.delete_cache("aeron-sample-embedded")?;

    // --- Counter operations ---
    println!("\n--- Counters ---");
    let counter_cache = "aeron-sample-counters";
    println!("Created counter cache: {}", client.create_counter_cache(counter_cache)?.cache_id);
    client.put_counter(counter_cache, "hits", 10)?;
    println!("increment hits +5 -> {}", client.increment_counter(counter_cache, "hits", 5)?.value);
    println!("decrement hits -3 -> {}", client.decrement_counter(counter_cache, "hits", 3)?.value);
    println!("set hits = 100 -> {}", client.set_counter(counter_cache, "hits", 100)?.value);

    // --- Timers ---
    // A timed entry schedules a pending TTL removal timer; getTimers streams all pending timers
    // (cache + counter) as one or more batches, reassembled here into a single list.
    println!("\n--- Timers ---");
    client.put_timed_item(cache_id, "expiring", "gone-soon", 600_000)?;
    for timer in client.get_timers()? {
        println!(
            "  [{}] {}/{} fires at {}",
            timer.timer_type, timer.cache_id, timer.key, timer.deadline
        );
    }

    client.delete_cache(cache_id)?;
    client.delete_counter_cache(counter_cache)?;
    println!("Done.");
    Ok(())
}
