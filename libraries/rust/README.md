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

Counter caches hold `i64` values and add `increment`, `decrement`, `set`, and timed-put operations:

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

Counter operations are also available via `bulk_ops` using the counter `BulkOperationType` variants and the `counter_value` field on `CacheOperationRequest`.

## Transports: HTTP+WS or Aeron

The library offers two transports for the same cache and counter operations:

- **HTTP + WebSocket** — [`AeronCacheClient`](src/lib.rs): REST for commands, WebSocket for streaming updates. Pure Rust, no native dependencies.
- **Aeron gateway** — [`AeronGatewayClient`](src/gateway.rs): a single low-latency, bidirectional Aeron connection carrying both commands and streaming updates, using the shared SBE wire protocol (`sbe/gateway-schema.xml`).

Enable the Aeron gateway on the backend with `AERON_TRANSPORT_GATEWAY_ENABLED=true`. By default it binds the request endpoint on port `7075` (stream `100`) and the response control endpoint on port `7076` (stream `101`).

> **Native build dependency:** the Aeron transport uses [`rusteron-client`](https://crates.io/crates/rusteron-client), which builds the Aeron C client. Building the crate therefore requires a C compiler, `cmake`, and `libclang` (for bindgen). The SBE codecs are pre-generated from the schema and vendored under `src/gateway_messages/`.

### Aeron usage

```rust
use aeron_cache_embedded_client::AeronGatewayClient;
use rusteron_media_driver::testing::EmbeddedDriver;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // An embedded media driver keeps the example self-contained; the gateway is reached over UDP.
    let driver = EmbeddedDriver::launch()?;
    let client = AeronGatewayClient::connect(driver.dir(), "127.0.0.1")?;
    client.await_connected(Duration::from_secs(10));

    client.create_cache("my-cache")?;
    client.put_item("my-cache", "key", "value")?;
    println!("{}", client.get_item("my-cache", "key")?.value);

    // Streaming updates over the same connection; dropping the handle unsubscribes.
    let sub = client.subscribe("my-cache", |e| println!("{} {:?}", e.event_type, e.item_key))?;
    client.put_item("my-cache", "streamed", "value")?;
    std::thread::sleep(Duration::from_millis(500));
    drop(sub);

    // Counters ride the same transport.
    client.create_counter_cache("counters")?;
    client.increment_counter("counters", "hits", 5)?;
    Ok(())
}
```

The Aeron transport also exposes operations not available over HTTP+WS: `get_cache_items` (full snapshot) and `get_stats`.
