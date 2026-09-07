//! Transport-neutral abstraction so the embedded caches ([`crate::EmbeddedCache`],
//! [`crate::EmbeddedCounters`]) work over either transport: the HTTP+WS [`crate::AeronCacheClient`]
//! or the Aeron [`crate::AeronGatewayClient`].
//!
//! [`CacheTransport`] exposes the common synchronous cache and counter operations plus
//! `subscribe_*_updates`, which delivers decoded update events to a callback in the background and
//! returns a [`CacheSubscription`] handle that unsubscribes when dropped — independent of whether the
//! underlying transport is a WebSocket or an Aeron subscription.

use crate::{
    AeronCacheClient, CacheUpdateEvent, CounterResponse, CounterUpdateEvent, CreateResponse,
    DeleteCacheResponse, DeleteItemResponse, GetItemResponse, PutItemResponse,
};
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

/// Callback types for streaming updates.
pub type CacheHandler = Arc<dyn Fn(CacheUpdateEvent) + Send + Sync>;
pub type CounterHandler = Arc<dyn Fn(CounterUpdateEvent) + Send + Sync>;

/// A handle to an active subscription. Dropping it unsubscribes / stops the background reader.
pub trait CacheSubscription: Send {}

/// Transport-neutral view of the Aeron Cache operations used by the embedded caches.
pub trait CacheTransport {
    // ---- cache ops ----
    fn put_item(&self, cache_id: &str, key: &str, value: &str) -> Result<PutItemResponse, Box<dyn Error>>;
    fn put_timed_item(&self, cache_id: &str, key: &str, value: &str, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>>;
    fn get_item(&self, cache_id: &str, key: &str) -> Result<GetItemResponse, Box<dyn Error>>;
    fn delete_item(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>>;
    fn delete_cache(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>>;

    // ---- counter ops ----
    fn create_counter_cache(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>>;
    fn put_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<PutItemResponse, Box<dyn Error>>;
    fn put_timed_counter(&self, cache_id: &str, key: &str, value: i64, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>>;
    fn get_counter(&self, cache_id: &str, key: &str) -> Result<CounterResponse, Box<dyn Error>>;
    fn increment_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>>;
    fn decrement_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>>;
    fn set_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<CounterResponse, Box<dyn Error>>;
    fn delete_counter(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>>;
    fn delete_counter_cache(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>>;

    // ---- subscriptions ----
    /// Subscribe to streaming updates for a cache; `handler` is invoked in the background per event.
    fn subscribe_cache_updates(&self, cache_id: &str, hydrate: bool, handler: CacheHandler) -> Result<Box<dyn CacheSubscription>, Box<dyn Error>>;
    /// Subscribe to streaming updates for a counter cache.
    fn subscribe_counter_updates(&self, cache_id: &str, hydrate: bool, handler: CounterHandler) -> Result<Box<dyn CacheSubscription>, Box<dyn Error>>;

    // ---- embedded caches ----
    /// A transport-neutral local-mirroring cache backed by this transport.
    fn embedded_cache(&self, cache_id: &str) -> crate::EmbeddedCache<'_>
    where
        Self: Sized,
    {
        crate::EmbeddedCache::new(self, cache_id.to_string())
    }

    /// A transport-neutral local-mirroring counter cache backed by this transport.
    fn embedded_counter_cache(&self, cache_id: &str) -> crate::EmbeddedCounters<'_>
    where
        Self: Sized,
    {
        crate::EmbeddedCounters::new(self, cache_id.to_string())
    }
}

// ------------------------------------------------------------------ HTTP+WS transport

/// A background WebSocket reader that drives update callbacks. Stops on drop.
pub struct WsSubscription {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl CacheSubscription for WsSubscription {}

impl Drop for WsSubscription {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Set a read timeout on the underlying TCP stream so the reader loop can observe the stop flag while
/// otherwise blocked waiting for data.
fn set_ws_read_timeout(socket: &mut WebSocket<MaybeTlsStream<std::net::TcpStream>>, timeout: Duration) {
    if let MaybeTlsStream::Plain(s) = socket.get_mut() {
        let _ = s.set_read_timeout(Some(timeout));
    }
}

/// Spawn a background thread that reads text frames from `socket`, reconnecting via `reconnect_url` on
/// error, and passes each frame's text to `on_text` until the returned subscription is dropped.
fn spawn_ws_reader<F>(
    mut socket: WebSocket<MaybeTlsStream<std::net::TcpStream>>,
    reconnect_url: String,
    on_text: F,
) -> WsSubscription
where
    F: Fn(&str) + Send + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    set_ws_read_timeout(&mut socket, Duration::from_millis(300));
    let handle = std::thread::spawn(move || {
        while !stop_thread.load(Ordering::Acquire) {
            match socket.read_message() {
                Ok(Message::Text(text)) => on_text(&text),
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // idle read timeout — loop to re-check the stop flag
                }
                Err(_) => {
                    if stop_thread.load(Ordering::Acquire) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(500));
                    if let Ok(url) = url::Url::parse(&reconnect_url) {
                        if let Ok((mut s, _)) = tungstenite::connect(url) {
                            set_ws_read_timeout(&mut s, Duration::from_millis(300));
                            socket = s;
                        }
                    }
                }
            }
        }
    });
    WsSubscription { stop, handle: Some(handle) }
}

impl CacheTransport for AeronCacheClient {
    fn put_item(&self, cache_id: &str, key: &str, value: &str) -> Result<PutItemResponse, Box<dyn Error>> {
        self.put_item(cache_id, key, value)
    }
    fn put_timed_item(&self, cache_id: &str, key: &str, value: &str, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        self.put_timed_item(cache_id, key, value, ttl)
    }
    fn get_item(&self, cache_id: &str, key: &str) -> Result<GetItemResponse, Box<dyn Error>> {
        self.get_item(cache_id, key)
    }
    fn delete_item(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        self.delete_item(cache_id, key)
    }
    fn delete_cache(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        self.delete_cache(cache_id)
    }

    fn create_counter_cache(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>> {
        self.create_counter_cache(cache_id)
    }
    fn put_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        self.put_counter(cache_id, key, value)
    }
    fn put_timed_counter(&self, cache_id: &str, key: &str, value: i64, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        self.put_timed_counter(cache_id, key, value, ttl)
    }
    fn get_counter(&self, cache_id: &str, key: &str) -> Result<CounterResponse, Box<dyn Error>> {
        self.get_counter(cache_id, key)
    }
    fn increment_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.increment_counter(cache_id, key, amount)
    }
    fn decrement_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.decrement_counter(cache_id, key, amount)
    }
    fn set_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.set_counter(cache_id, key, value)
    }
    fn delete_counter(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        self.delete_counter(cache_id, key)
    }
    fn delete_counter_cache(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        self.delete_counter_cache(cache_id)
    }

    fn subscribe_cache_updates(&self, cache_id: &str, hydrate: bool, handler: CacheHandler) -> Result<Box<dyn CacheSubscription>, Box<dyn Error>> {
        let socket = self.subscribe_ext(cache_id, hydrate)?;
        let base = self.ws_url.trim_end_matches('/').to_string();
        let reconnect_url = format!("{base}/api/ws/v1/cache/{cache_id}");
        let sub = spawn_ws_reader(socket, reconnect_url, move |text| {
            if let Ok(event) = serde_json::from_str::<CacheUpdateEvent>(text) {
                handler(event);
            }
        });
        Ok(Box::new(sub))
    }

    fn subscribe_counter_updates(&self, cache_id: &str, hydrate: bool, handler: CounterHandler) -> Result<Box<dyn CacheSubscription>, Box<dyn Error>> {
        let socket = self.subscribe_counter_ext(cache_id, hydrate)?;
        let base = self.ws_url.trim_end_matches('/').to_string();
        let reconnect_url = format!("{base}/api/ws/v1/counter/{cache_id}");
        let sub = spawn_ws_reader(socket, reconnect_url, move |text| {
            if let Ok(event) = serde_json::from_str::<CounterUpdateEvent>(text) {
                handler(event);
            }
        });
        Ok(Box::new(sub))
    }
}

// ------------------------------------------------------------------ Aeron gateway transport

use crate::gateway::{AeronGatewayClient, GatewaySubscription};

impl CacheSubscription for GatewaySubscription {}

impl CacheTransport for AeronGatewayClient {
    fn put_item(&self, cache_id: &str, key: &str, value: &str) -> Result<PutItemResponse, Box<dyn Error>> {
        self.put_item(cache_id, key, value)
    }
    fn put_timed_item(&self, cache_id: &str, key: &str, value: &str, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        self.put_timed_item(cache_id, key, value, ttl)
    }
    fn get_item(&self, cache_id: &str, key: &str) -> Result<GetItemResponse, Box<dyn Error>> {
        self.get_item(cache_id, key)
    }
    fn delete_item(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        self.delete_item(cache_id, key)
    }
    fn delete_cache(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        self.delete_cache(cache_id)
    }

    fn create_counter_cache(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>> {
        self.create_counter_cache(cache_id)
    }
    fn put_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        self.put_counter(cache_id, key, value)
    }
    fn put_timed_counter(&self, cache_id: &str, key: &str, value: i64, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        self.put_timed_counter(cache_id, key, value, ttl)
    }
    fn get_counter(&self, cache_id: &str, key: &str) -> Result<CounterResponse, Box<dyn Error>> {
        self.get_counter(cache_id, key)
    }
    fn increment_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.increment_counter(cache_id, key, amount)
    }
    fn decrement_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.decrement_counter(cache_id, key, amount)
    }
    fn set_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.set_counter(cache_id, key, value)
    }
    fn delete_counter(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        self.delete_counter(cache_id, key)
    }
    fn delete_counter_cache(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        self.delete_counter_cache(cache_id)
    }

    fn subscribe_cache_updates(&self, cache_id: &str, hydrate: bool, handler: CacheHandler) -> Result<Box<dyn CacheSubscription>, Box<dyn Error>> {
        let sub = self.subscribe_ext(cache_id, hydrate, move |event| handler(event))?;
        Ok(Box::new(sub))
    }

    fn subscribe_counter_updates(&self, cache_id: &str, hydrate: bool, handler: CounterHandler) -> Result<Box<dyn CacheSubscription>, Box<dyn Error>> {
        let sub = self.subscribe_counter_ext(cache_id, hydrate, move |event| handler(event))?;
        Ok(Box::new(sub))
    }
}
