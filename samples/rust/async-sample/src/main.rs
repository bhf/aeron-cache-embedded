use aeron_cache_embedded_client::AeronCacheClient;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let default_url = "http://localhost:7070".to_string();
    let ws_url = "ws://localhost:7071".to_string();
    let base_url = args.get(1).unwrap_or(&default_url);
    let ws_url = args.get(2).unwrap_or(&ws_url);

    println!("Starting Async Sample against {}, wsUrl: {}", base_url, ws_url);

    let client = AeronCacheClient::new(base_url.clone(), ws_url.clone());
    let cache_id = "async-sample";

    println!("Will now create cache id {}", cache_id);

    // Ensure the cache exists on the server
    let _ = client.create_cache_async(cache_id).await;

    // Get the handle
    let cache = client.get_cache(cache_id);

    println!("Putting key 'async-key' -> 'async-value' asynchronously");
    
    // Spawn the put
    let put_future = cache.insert_async("async-key", "async-value");

    // Await it
    put_future.await?;
    println!("Put operation completed.");

    // Allow propagation
    sleep(Duration::from_millis(100)).await;

    match cache.get_async("async-key").await {
        Ok(val) => println!("Read key 'async-key': {}", val),
        Err(e) => println!("Read key 'async-key' error: {}", e),
    }

    Ok(())
}
