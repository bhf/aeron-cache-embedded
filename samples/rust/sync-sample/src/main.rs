use aeron_cache_embedded_client::embedded_cache::EmbeddedAeronCache;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let default_url = "http://localhost:8080".to_string();
    let base_url = args.get(1).unwrap_or(&default_url);

    println!("Starting Sync Sample against {}", base_url);

    let mut cache = EmbeddedAeronCache::new(base_url.clone());
    cache.connect().await?;

    println!("Putting key 'sync-key' -> 'sync-value'");
    cache.put("sync-key", "sync-value".to_string()).await?;

    // Allow propagation
    sleep(Duration::from_millis(100)).await;

    let val = cache.get("sync-key").await;
    match val {
        Some(v) => println!("Read back key 'sync-key': {}", v),
        None => println!("Key 'sync-key' not found in local cache yet."),
    }

    Ok(())
}
