use aeron_cache_embedded_client::{AeronCacheClient, CacheTransport, CacheUpdateEvent};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Demonstrates server-side synthetic PATCH streaming.
///
/// Writes evolving *full* JSON values with ordinary `put_item` calls (never `patch_item`). As long as a
/// patch-mode subscriber is listening, the cache service turns those puts into patch traffic: the first put
/// of a key arrives as a full `ADD_ITEM` baseline, each later overwrite as a `PATCH_ITEM` delta of only the
/// changed fields, an identical (no-op) put is suppressed, and a non-JSON value falls back to a full
/// `ADD_ITEM`. The `EmbeddedObjects` mirror deep-merges each delta (RFC 7386) so `get` always holds
/// the full, reconstructed object.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let base_url = args.get(1).cloned().unwrap_or_else(|| "http://localhost:7070".to_string());
    let ws_url = args.get(2).cloned().unwrap_or_else(|| "ws://localhost:7071".to_string());

    println!("Starting Patch Stream Sample against {}, ws={}", base_url, ws_url);

    let client = AeronCacheClient::new(base_url, ws_url);
    let cache_id = "patch-stream-sample-cache";
    let _ = client.create_cache_async(cache_id).await;

    // Start from a clean slate so the first put is a fresh insert (an ADD_ITEM baseline) rather than an
    // overwrite of a value left over from a previous run (which would arrive as a PATCH_ITEM delta).
    let _ = client.delete_item_async(cache_id, "doc").await;

    // Subscribe BEFORE the key exists. One patch-mode subscription just logs the raw events the server
    // sends (the ADD_ITEM baseline and the PATCH_ITEM deltas)...
    let _log_sub = client.subscribe_cache_updates_filtered(
        cache_id,
        false,
        None,
        Some("patch"),
        Arc::new(|ev: CacheUpdateEvent| {
            if ev.item_key.as_deref() == Some("doc") {
                println!("[Rust] <- {} delta={}", ev.event_type, ev.item_value.unwrap_or_default());
            }
        }),
    )?;

    // ...and a second patch-mode subscription via the EmbeddedObjects mirror deep-merges those deltas so
    // get reconstructs the full object.
    let objects = client.embedded_object_cache(cache_id);
    let _mirror = objects.subscribe_filtered(false, None, Some("patch"))?;

    let states = [
        r#"{"user":{"name":"ada","age":36},"status":"active"}"#,
        r#"{"user":{"name":"ada","age":37},"status":"active"}"#, // delta: user.age -> 37
        r#"{"user":{"name":"ada","age":37},"status":"away"}"#,   // delta: status -> away
        r#"{"user":{"name":"ada","age":37},"status":"away"}"#,   // identical -> suppressed
        r#"{"user":{"name":"ada","age":37}}"#,                     // delta: status -> null (removed)
    ];

    sleep(Duration::from_millis(800)).await; // let the subscriptions register before the first put

    let mut i = 0usize;
    loop {
        let state = states[i % states.len()];
        println!("[Rust] -> put doc = {}", state);
        client.put_item_async(cache_id, "doc", state).await?;
        sleep(Duration::from_millis(400)).await;
        match objects.get("doc") {
            Some(v) => println!("[Rust]    reconstructed={}", v),
            None => println!("[Rust]    reconstructed=<none>"),
        }
        i += 1;
        sleep(Duration::from_millis(2100)).await;
    }
}
