//! Transport-neutral embedded caches that keep a local mirror in sync from streaming updates over
//! either transport (HTTP+WS or Aeron). Obtain one from any [`crate::CacheTransport`] via
//! `embedded_cache(..)` / `embedded_counter_cache(..)`, e.g.
//!
//! ```no_run
//! use aeron_cache_embedded_client::{AeronCacheClient, CacheTransport};
//! let client = AeronCacheClient::new("http://localhost:7070".into(), "ws://localhost:7071".into());
//! let cache = client.embedded_cache("my-cache");
//! let _sub = cache.subscribe().unwrap(); // local mirror now updates in the background
//! cache.insert("k", "v").unwrap();
//! ```

use crate::transport::{CacheHandler, CacheSubscription, CacheTransport, CounterHandler};
use crate::{
    CacheUpdateEvent, CounterResponse, CounterUpdateEvent, DeleteCacheResponse, DeleteItemResponse,
    GetItemResponse, PutItemResponse,
};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, RwLock};

/// A cache view that mirrors remote updates into a local map, backed by any [`CacheTransport`].
pub struct EmbeddedCache<'a> {
    transport: &'a dyn CacheTransport,
    cache_id: String,
    local: Arc<RwLock<HashMap<String, String>>>,
}

impl<'a> EmbeddedCache<'a> {
    pub fn new(transport: &'a dyn CacheTransport, cache_id: String) -> Self {
        EmbeddedCache { transport, cache_id, local: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Read a value from the local mirror (no network round-trip).
    pub fn get_local(&self, key: &str) -> Option<String> {
        self.local.read().ok().and_then(|m| m.get(key).cloned())
    }

    /// Snapshot of the local mirror.
    pub fn local_snapshot(&self) -> HashMap<String, String> {
        self.local.read().map(|m| m.clone()).unwrap_or_default()
    }

    pub fn insert(&self, key: &str, value: &str) -> Result<PutItemResponse, Box<dyn Error>> {
        self.transport.put_item(&self.cache_id, key, value)
    }

    pub fn insert_timed(&self, key: &str, value: &str, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        self.transport.put_timed_item(&self.cache_id, key, value, ttl)
    }

    pub fn get(&self, key: &str) -> Result<GetItemResponse, Box<dyn Error>> {
        self.transport.get_item(&self.cache_id, key)
    }

    pub fn remove(&self, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        self.transport.delete_item(&self.cache_id, key)
    }

    pub fn clear(&self) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        self.transport.delete_cache(&self.cache_id)
    }

    /// Subscribe to streaming updates; the local mirror is kept in sync in the background until the
    /// returned handle is dropped.
    pub fn subscribe(&self) -> Result<Box<dyn CacheSubscription>, Box<dyn Error>> {
        self.subscribe_ext(false)
    }

    pub fn subscribe_ext(&self, hydrate: bool) -> Result<Box<dyn CacheSubscription>, Box<dyn Error>> {
        let local = self.local.clone();
        let handler: CacheHandler = Arc::new(move |event: CacheUpdateEvent| update_cache_local(&local, event));
        self.transport.subscribe_cache_updates(&self.cache_id, hydrate, handler)
    }
}

fn update_cache_local(local: &Arc<RwLock<HashMap<String, String>>>, event: CacheUpdateEvent) {
    if let Ok(mut m) = local.write() {
        match event.event_type.as_str() {
            "ADD_ITEM" => {
                if let (Some(k), Some(v)) = (event.item_key, event.item_value) {
                    m.insert(k, v);
                }
            }
            "REMOVE_ITEM" => {
                if let Some(k) = event.item_key {
                    m.remove(&k);
                }
            }
            "CLEAR_CACHE" | "DELETE_CACHE" => m.clear(),
            _ => {}
        }
    }
}

/// A counter-cache view that mirrors remote updates into a local map, backed by any [`CacheTransport`].
pub struct EmbeddedCounters<'a> {
    transport: &'a dyn CacheTransport,
    cache_id: String,
    local: Arc<RwLock<HashMap<String, i64>>>,
}

impl<'a> EmbeddedCounters<'a> {
    pub fn new(transport: &'a dyn CacheTransport, cache_id: String) -> Self {
        EmbeddedCounters { transport, cache_id, local: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub fn get_local(&self, key: &str) -> Option<i64> {
        self.local.read().ok().and_then(|m| m.get(key).copied())
    }

    pub fn local_snapshot(&self) -> HashMap<String, i64> {
        self.local.read().map(|m| m.clone()).unwrap_or_default()
    }

    pub fn insert(&self, key: &str, value: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        self.transport.put_counter(&self.cache_id, key, value)
    }

    pub fn insert_timed(&self, key: &str, value: i64, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        self.transport.put_timed_counter(&self.cache_id, key, value, ttl)
    }

    pub fn get(&self, key: &str) -> Result<CounterResponse, Box<dyn Error>> {
        self.transport.get_counter(&self.cache_id, key)
    }

    pub fn increment(&self, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.transport.increment_counter(&self.cache_id, key, amount)
    }

    pub fn decrement(&self, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.transport.decrement_counter(&self.cache_id, key, amount)
    }

    pub fn set(&self, key: &str, value: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.transport.set_counter(&self.cache_id, key, value)
    }

    pub fn remove(&self, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        self.transport.delete_counter(&self.cache_id, key)
    }

    pub fn clear(&self) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        self.transport.delete_counter_cache(&self.cache_id)
    }

    pub fn subscribe(&self) -> Result<Box<dyn CacheSubscription>, Box<dyn Error>> {
        self.subscribe_ext(false)
    }

    pub fn subscribe_ext(&self, hydrate: bool) -> Result<Box<dyn CacheSubscription>, Box<dyn Error>> {
        let local = self.local.clone();
        let handler: CounterHandler = Arc::new(move |event: CounterUpdateEvent| update_counter_local(&local, event));
        self.transport.subscribe_counter_updates(&self.cache_id, hydrate, handler)
    }
}

fn update_counter_local(local: &Arc<RwLock<HashMap<String, i64>>>, event: CounterUpdateEvent) {
    if let Ok(mut m) = local.write() {
        match event.event_type.as_str() {
            "ADD_ITEM" => {
                if let (Some(k), Some(v)) = (event.item_key, event.item_value) {
                    m.insert(k, v);
                }
            }
            "REMOVE_ITEM" => {
                if let Some(k) = event.item_key {
                    m.remove(&k);
                }
            }
            "CLEAR_CACHE" | "DELETE_CACHE" => m.clear(),
            _ => {}
        }
    }
}
