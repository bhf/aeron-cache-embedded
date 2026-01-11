# Aeron Cache Rust Client

This is the Rust client library for Aeron Cache.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
aeron-cache-embedded-client = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## Usage

```rust
use aeron_cache_embedded_client::AeronCacheClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AeronCacheClient::new(
        "http://localhost:7070".to_string(), 
        "ws://localhost:7071".to_string()
    );
    let cache = client.get_cache("my-cache");

    // Remote writes (async)
    cache.insert_async("key", "value").await?;

    // Local reads (from embedded cache)
    println!("{:?}", cache.get_local("key"));

    Ok(())
}
```
