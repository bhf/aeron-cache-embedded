use aeron_cache_embedded_client::AeronCacheClient;
use std::env;
use std::time::Duration;
use tokio::task;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let default_url = "http://localhost:7070".to_string();
    let ws_url = "ws://localhost:7071".to_string();
    let base_url = args.get(1).unwrap_or(&default_url);
    let ws_url = args.get(2).unwrap_or(&ws_url);

    println!("Starting Streaming Sample against {}, wsUrl: {}", base_url, ws_url);

    let client = AeronCacheClient::new(base_url.clone(), ws_url.clone());
    let cache_id = "streaming-sample-cache";

    println!("Will now create cache id {}", cache_id);

    // Ensure the cache exists on the server
    let _ = client.create_cache_async(cache_id).await;

    // Get the handle
    let cache = client.get_cache(cache_id);

    println!("Connected. Waiting for updates on '{}'...", cache_id);

    // Subscribe to updates. This creates a new WebSocket connection.
    let mut socket = cache.subscribe()?;

    // Spawn a blocking task to handle WebSocket updates
    task::spawn_blocking(move || {
        loop {
            match socket.read_message() {
                Ok(msg) => {
                    if msg.is_text() || msg.is_binary() {
                        println!("Observed change: {}", msg);
                    }
                }
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    });

    println!("Poller started for 'streaming-key'...");

    loop {
        let current = cache.get_local("streaming-key");
        println!("Polled 'streaming-key': {:?}", current);
        sleep(Duration::from_secs(1)).await;
    }
}
