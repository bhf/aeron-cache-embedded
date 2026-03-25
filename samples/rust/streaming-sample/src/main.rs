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
    let create_response = client.create_cache_async(cache_id).await?;
    println!("Created cache: {}", create_response.cache_id);

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
                        let text = msg.to_text().unwrap_or("");
                        println!("[Rust] Observed change: {}", text);
                    }
                }
                Err(e) => {
                    eprintln!("[Rust] WebSocket error: {}", e);
                    break;
                }
            }
        }
    });

    println!("[Rust] Poller started for 'streaming-key'...");

    let mut last_val = None;
    loop {
        if let Some(current) = cache.get_local("streaming-key") {
            if Some(current.clone()) != last_val {
                println!("[Rust-Poller] Observed change in 'streaming-key': {}", current);
                last_val = Some(current);
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
}
