use aeron_cache_embedded_client::embedded_cache::EmbeddedAeronCache;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let default_url = "http://localhost:8080".to_string();
    let base_url = args.get(1).unwrap_or(&default_url);

    println!("Starting Streaming Sample against {}", base_url);

    let mut cache = EmbeddedAeronCache::new(base_url.clone());
    cache.connect().await?;

    println!("Connected. Waiting for updates on 'shared-key'...");

    let mut last_value = String::new();

    loop {
        let current = cache.get("shared-key").await.unwrap_or_default();
        if current != last_value && !current.is_empty() {
             println!("Observed change in 'shared-key': {}", current);
             last_value = current;
        }
        sleep(Duration::from_secs(1)).await;
    }
}
