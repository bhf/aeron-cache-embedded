use aeron_cache_embedded_client::embedded_cache::EmbeddedAeronCache;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let default_url = "http://localhost:8080".to_string();
    let base_url = args.get(1).unwrap_or(&default_url);

    println!("Starting Async Sample against {}", base_url);

    let mut cache = EmbeddedAeronCache::new(base_url.clone());
    cache.connect().await?;

    println!("Putting key 'async-key' -> 'async-value' asynchronously");
    
    // Spawn the put
    let put_future = cache.put("async-key", "async-value".to_string());
    
    // Await it
    put_future.await?;
    println!("Put operation completed.");

    // Allow propagation
    sleep(Duration::from_millis(100)).await;

    let val = cache.get("async-key").await;
    println!("Read key 'async-key': {}", val.unwrap_or("not found".to_string()));

    Ok(())
}
