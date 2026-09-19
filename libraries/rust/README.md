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

## Inspection & management operations

Beyond the basic CRUD, the HTTP client exposes operations for inspecting and managing caches and
counters. Every method has an `_async` variant (e.g. `get_caches` / `get_caches_async`).

```rust
use aeron_cache_embedded_client::AeronCacheClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AeronCacheClient::new(
        "http://localhost:7070".to_string(),
        "ws://localhost:7071".to_string(),
    );

    // Deep-merge JSON into an existing item.
    client.patch_item("my-cache", "doc", r#"{"b":2}"#)?;

    // Keep a timed item that was scheduled for removal.
    client.cancel_item_removal("my-cache", "doc")?;

    // Enumerate caches and read server-wide statistics.
    for details in client.get_caches()? {
        println!("{} ({} items)", details.cache_id, details.item_count);
    }
    let stats = client.get_stats()?;
    println!("caches={} items={}", stats.total_caches_count, stats.total_items_count);

    // Counter equivalents.
    let counters = client.get_counter_items("my-counters")?;   // full snapshot
    println!("{} counters", counters.items.len());
    client.clear_counter_cache("my-counters")?;
    client.cancel_counter_item_removal("my-counters", "hits")?;
    let counter_caches = client.get_counter_caches()?;
    let counter_stats = client.get_counter_stats()?;
    println!("{} counter caches, {} items", counter_caches.len(), counter_stats.total_items_count);

    Ok(())
}
```

| Cache | Counter | HTTP |
| --- | --- | --- |
| `patch_item` | — | `PATCH /api/v1/cache/{id}/{key}` |
| `cancel_item_removal` | `cancel_counter_item_removal` | `POST .../{key}/cancel-removal` |
| `get_cache_items` | `get_counter_items` | `GET /api/v1/cache/{id}` · `GET /api/v1/counters/{id}` |
| `clear_cache` | `clear_counter_cache` | `PATCH /api/v1/cache/{id}` · `PATCH /api/v1/counters/{id}` |
| `get_caches` | `get_counter_caches` | `GET /api/v1/caches` · `GET /api/v1/counters-caches` |
| `get_stats` | `get_counter_stats` | `GET /api/v1/stats` · `GET /api/v1/counters-stats` |

## Transports: HTTP+WS or Aeron

The library offers three transports for the same cache and counter operations:

- **HTTP + WebSocket** — [`AeronCacheClient`](src/lib.rs): REST for commands, WebSocket for streaming updates. Pure Rust, no native dependencies.
- **Bidirectional WebSocket** — [`AeronBidiClient`](src/bidi.rs): a single WebSocket connection to `/api/ws/v1/bidi` carrying both commands and streaming updates as JSON frames. Pure Rust, no native dependencies.
- **Aeron gateway** — [`AeronGatewayClient`](src/gateway.rs): a single low-latency, bidirectional Aeron connection carrying both commands and streaming updates, using the shared SBE wire protocol (`sbe/gateway-schema.xml`).

Enable the Aeron gateway on the backend with `AERON_GATEWAY_ENABLED=true`. By default it binds the request endpoint on port `7075` (stream `100`) and the response control endpoint on port `7076` (stream `101`).

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

### Bidirectional WebSocket usage

[`AeronBidiClient`](src/bidi.rs) multiplexes the full cache + counter command surface plus dynamic
subscribe/unsubscribe over a single WebSocket connection, correlated by a client-minted
`correlationId`. It is a pure-Rust alternative to the Aeron gateway (no native build dependencies),
with a synchronous API mirroring the HTTP client: a background reader thread dispatches frames and
correlates single responses over mpsc channels. Stream updates are routed to listeners **by
`cacheId`** (the server stamps `streamUpdate` with the causing command's id, not the subscription's).

```rust
use aeron_cache_embedded_client::AeronBidiClient;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connects to {ws_url}/api/ws/v1/bidi.
    let client = AeronBidiClient::connect("ws://localhost:7071")?;

    client.create_cache("my-cache")?;
    client.put_item("my-cache", "key", "value")?;
    println!("{}", client.get_item("my-cache", "key")?.value);

    // Streaming updates over the same connection; dropping (or closing) the handle unsubscribes.
    let sub = client.subscribe("my-cache", |e| println!("{} {:?}", e.event_type, e.item_key))?;
    client.put_item("my-cache", "streamed", "value")?;
    std::thread::sleep(Duration::from_millis(500));
    sub.close()?;

    // Counters ride the same connection.
    client.create_counter_cache("counters")?;
    client.increment_counter("counters", "hits", 5)?;
    Ok(())
}
```

Like the Aeron transport, the bidi client also exposes `get_cache_items` / `get_counter_items`
(full snapshots) and `get_stats` / `get_counter_stats`, which return a `Vec<StatEntry>` (one entry
per cache) rather than the HTTP aggregate `CacheStatsResponse`.

### Transport-neutral embedded caches

Both clients implement the [`CacheTransport`](src/transport.rs) trait, so the local-mirroring
`EmbeddedCache` / `EmbeddedCounters` work identically over either transport. Call `embedded_cache(..)`
on either client; `subscribe()` starts a background reader (a WebSocket reader over HTTP, an Aeron
subscription over the gateway) that keeps the local mirror in sync. Reads are served locally with no
network round-trip.

```rust
use aeron_cache_embedded_client::CacheTransport;

// `client` may be an AeronCacheClient (HTTP+WS) or an AeronGatewayClient (Aeron).
let cache = client.embedded_cache("my-cache");
let _sub = cache.subscribe()?;            // mirror updates in the background until `_sub` is dropped
cache.insert("key", "value")?;
std::thread::sleep(std::time::Duration::from_millis(500));
println!("{:?}", cache.get_local("key")); // Some("value") — from the local mirror
```

The HTTP-specific `EmbeddedAeronCache` / `EmbeddedCounterCache` (with `subscribe()` returning a
user-polled `UpdatingWebSocket`, plus async methods) remain available via `get_cache` / `get_counter_cache`.
