use aeron_cache_embedded_client::AeronCacheClient;
use std::env;
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let default_url = "http://localhost:7070".to_string();
    let ws_url = "ws://localhost:7071".to_string();
    let base_url = args.get(1).unwrap_or(&default_url);
    let ws_url = args.get(2).unwrap_or(&ws_url);

    println!("Starting Sync Sample against {}, wsUrl {}", base_url, ws_url);

    let client = AeronCacheClient::new(base_url.clone(), ws_url.clone());
    let cache_id = "sync-sample";

    // Ensure the cache exists on the server (Sync)
    let create_response = client.create_cache(cache_id)?;
    println!("Created cache: {}", create_response.cache_id);

    // Get the handle
    let cache = client.get_cache(cache_id);

    println!("Putting key 'sync-key' -> 'sync-value' synchronously");
    let put_response = cache.insert("sync-key", "sync-value")?;
    println!("Put operation status: {}", put_response.status);

    // Allow propagation
    sleep(Duration::from_millis(100));

    match cache.get("sync-key") {
        Ok(resp) => println!("Read back key 'sync-key': {}", resp.value),
        Err(e) => println!("Read key 'sync-key' error: {}", e),
    }

    println!("Putting key 'timed-key' -> 'timed-value' with 5000ms TTL");
    let timed_put_response = cache.insert_timed("timed-key", "timed-value", 5000)?;
    println!("Timed Put operation status: {}", timed_put_response.operation_status);

    match cache.get("timed-key") {
        Ok(resp) => println!("Read back key 'timed-key': {}", resp.value),
        Err(e) => println!("Read key 'timed-key' error: {}", e),
    }

    // --- Counter operations ---
    let counter_cache_id = "sync-counter-sample";
    let counter_create = client.create_counter_cache(counter_cache_id)?;
    println!("Created counter cache: {}", counter_create.cache_id);

    let counters = client.get_counter_cache(counter_cache_id);

    println!("Putting counter 'requests' -> 10 synchronously");
    counters.insert("requests", 10)?;
    println!("Incremented 'requests' by 5 -> {}", counters.increment("requests", 5)?.value);
    println!("Decremented 'requests' by 3 -> {}", counters.decrement("requests", 3)?.value);
    println!("Set 'requests' -> {}", counters.set("requests", 100)?.value);
    println!("Read counter 'requests': {}", counters.get("requests")?.value);

    // --- Inspection & management operations ---
    println!("Patching 'doc' (deep-merge)");
    client.put_item(cache_id, "doc", r#"{"a":1}"#)?;
    let patch_resp = client.patch_item(cache_id, "doc", r#"{"b":2}"#)?;
    println!("Patch status: {}", patch_resp.operation_status);
    println!("Doc after patch: {}", client.get_item(cache_id, "doc")?.value);

    println!("Listing all caches:");
    for details in client.get_caches()? {
        println!("  - {} ({} items)", details.cache_id, details.item_count);
    }

    let stats = client.get_stats()?;
    println!(
        "Cache stats: caches={} items={} ops={} errors={}",
        stats.total_caches_count, stats.total_items_count, stats.total_ops_count, stats.error_count
    );

    // A timed entry schedules a pending TTL removal timer; getTimers lists all pending timers
    // across both caches and counter caches, each tagged CACHE or COUNTER.
    client.put_timed_item(cache_id, "expiring", "gone-soon", 600_000)?;
    println!("Listing all pending TTL timers:");
    for timer in client.get_timers()?.timers {
        println!(
            "  - [{}] {}/{} fires at {}",
            timer.timer_type, timer.cache_id, timer.key, timer.deadline
        );
    }

    println!("Listing all counters in '{}':", counter_cache_id);
    for item in client.get_counter_items(counter_cache_id)?.items {
        println!("  - {} = {}", item.key, item.value);
    }

    let counter_stats = client.get_counter_stats()?;
    println!(
        "Counter stats: caches={} items={}",
        counter_stats.total_caches_count, counter_stats.total_items_count
    );

    Ok(())
}
