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

## Counters

Counter caches hold `i64` values and add `increment`, `decrement`, and `set` operations:

```rust
use aeron_cache_embedded_client::AeronCacheClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AeronCacheClient::new(
        "http://localhost:7070".to_string(),
        "ws://localhost:7071".to_string(),
    );
    client.create_counter_cache("counter-cache")?;
    let counters = client.get_counter_cache("counter-cache");

    counters.insert("requests", 10)?;
    counters.increment("requests", 5)?; // -> 15
    counters.decrement("requests", 3)?; // -> 12
    counters.set("requests", 100)?;     // -> 100
    println!("{}", counters.get("requests")?.value);

    Ok(())
}
```

## Bulk Operations

```rust
use aeron_cache_embedded_client::{BulkCacheOpsRequest, CacheOperationRequest, bulk_operation_type};

let request = BulkCacheOpsRequest {
    request_id: Some("req-1".to_string()),
    operations: vec![CacheOperationRequest {
        operation_type: bulk_operation_type::INCREMENT_COUNTER.to_string(),
        cache_id: Some("counter-cache".to_string()),
        key: Some("requests".to_string()),
        counter_value: Some(5),
        ..Default::default()
    }],
};
let result = client.bulk_ops(&request)?;
```
