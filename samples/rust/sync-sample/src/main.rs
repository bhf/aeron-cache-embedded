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
    let _ = client.create_cache(cache_id);

    // Get the handle
    let cache = client.get_cache(cache_id);

    println!("Putting key 'sync-key' -> 'sync-value' synchronously");
    cache.insert("sync-key", "sync-value")?;

    println!("Put operation completed.");

    // Allow propagation
    sleep(Duration::from_millis(100));

    match cache.get("sync-key") {
        Ok(val) => println!("Read back key 'sync-key': {}", val),
        Err(e) => println!("Read key 'sync-key' error: {}", e),
    }

    Ok(())
}
