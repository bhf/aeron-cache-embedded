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
    GetItemResponse, PatchItemResponse, PutItemResponse,
};
use serde_json::Value;
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
            // PATCH_ITEM is intentionally ignored: it carries only the changed fields (a delta), so
            // applying it to this string mirror would overwrite the full stored value with the fragment
            // and lose the untouched fields. Use `EmbeddedObjects`, which deep-merges deltas, for patch
            // mode.
            "PATCH_ITEM" => {}
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

/// A cache view whose values are structured JSON objects rather than opaque strings, backed by any
/// [`CacheTransport`].
///
/// The key difference from [`EmbeddedCache`] is how `PATCH_ITEM` events are applied: instead of
/// overwriting the entry with the delta, the delta is deep-merged into the stored object using RFC 7386
/// (JSON Merge Patch) semantics — nested objects merge recursively, scalars and arrays are replaced, and
/// a `null` field in the delta deletes that field. This matches the server-side [`CacheTransport::patch_item`]
/// deep-merge, so a patch-mode subscription (which streams only the changed fields) reconstructs the full
/// object locally without losing untouched fields.
pub struct EmbeddedObjects<'a> {
    transport: &'a dyn CacheTransport,
    cache_id: String,
    local: Arc<RwLock<HashMap<String, Value>>>,
}

impl<'a> EmbeddedObjects<'a> {
    pub fn new(transport: &'a dyn CacheTransport, cache_id: String) -> Self {
        EmbeddedObjects { transport, cache_id, local: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Read an object from the local mirror (no network round-trip).
    pub fn get_local(&self, key: &str) -> Option<Value> {
        self.local.read().ok().and_then(|m| m.get(key).cloned())
    }

    /// Snapshot of the local mirror.
    pub fn local_snapshot(&self) -> HashMap<String, Value> {
        self.local.read().map(|m| m.clone()).unwrap_or_default()
    }

    pub fn insert(&self, key: &str, value: &Value) -> Result<PutItemResponse, Box<dyn Error>> {
        self.transport.put_item(&self.cache_id, key, &value.to_string())
    }

    pub fn insert_timed(&self, key: &str, value: &Value, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        self.transport.put_timed_item(&self.cache_id, key, &value.to_string(), ttl)
    }

    /// Deep-merge a JSON fragment into the stored object (RFC 7386). A `null` field deletes it.
    pub fn patch(&self, key: &str, fragment: &Value) -> Result<PatchItemResponse, Box<dyn Error>> {
        self.transport.patch_item(&self.cache_id, key, &fragment.to_string())
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

    /// Subscribe to streaming updates; the local mirror is deep-merged in the background until the
    /// returned handle is dropped.
    pub fn subscribe(&self) -> Result<Box<dyn CacheSubscription>, Box<dyn Error>> {
        self.subscribe_ext(false)
    }

    pub fn subscribe_ext(&self, hydrate: bool) -> Result<Box<dyn CacheSubscription>, Box<dyn Error>> {
        let local = self.local.clone();
        let handler: CacheHandler = Arc::new(move |event: CacheUpdateEvent| update_object_local(&local, event));
        self.transport.subscribe_cache_updates(&self.cache_id, hydrate, handler)
    }
}

fn update_object_local(local: &Arc<RwLock<HashMap<String, Value>>>, event: CacheUpdateEvent) {
    if let Ok(mut m) = local.write() {
        match event.event_type.as_str() {
            "ADD_ITEM" => {
                if let (Some(k), Some(v)) = (event.item_key, event.item_value) {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&v) {
                        if parsed.is_object() {
                            m.insert(k, parsed);
                        }
                    }
                }
            }
            "PATCH_ITEM" => {
                if let (Some(k), Some(v)) = (event.item_key, event.item_value) {
                    if let Ok(delta) = serde_json::from_str::<Value>(&v) {
                        if delta.is_object() {
                            let base = m.remove(&k).unwrap_or_else(|| Value::Object(serde_json::Map::new()));
                            m.insert(k, deep_merge(base, delta));
                        }
                    }
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

/// RFC 7386 (JSON Merge Patch) deep-merge of `patch` into `target`, returning the merged value. Nested
/// objects merge recursively; scalars and arrays replace; a `null` field deletes the field.
fn deep_merge(target: Value, patch: Value) -> Value {
    let patch_obj = match patch {
        Value::Object(o) => o,
        // A non-object patch replaces the target entirely.
        other => return other,
    };
    let mut base = match target {
        Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };
    for (k, v) in patch_obj {
        if v.is_null() {
            base.remove(&k);
        } else if v.is_object() {
            let existing = base.remove(&k).unwrap_or_else(|| Value::Object(serde_json::Map::new()));
            base.insert(k, deep_merge(existing, v));
        } else {
            base.insert(k, v);
        }
    }
    Value::Object(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(event_type: &str, key: Option<&str>, value: Option<&str>) -> CacheUpdateEvent {
        CacheUpdateEvent {
            cache_id: "test-cache".to_string(),
            event_type: event_type.to_string(),
            item_key: key.map(|s| s.to_string()),
            item_value: value.map(|s| s.to_string()),
            request_id: "123".to_string(),
        }
    }

    #[test]
    fn deep_merge_recurses_and_preserves_untouched_fields() {
        let merged = deep_merge(json!({"a":1,"b":{"c":2}}), json!({"b":{"d":3}}));
        assert_eq!(merged, json!({"a":1,"b":{"c":2,"d":3}}));
    }

    #[test]
    fn deep_merge_null_deletes_field() {
        let merged = deep_merge(json!({"a":1,"b":2}), json!({"b":null}));
        assert_eq!(merged, json!({"a":1}));
    }

    #[test]
    fn deep_merge_replaces_scalars_and_arrays() {
        let merged = deep_merge(json!({"n":1,"list":[1,2,3]}), json!({"n":9,"list":[4]}));
        assert_eq!(merged, json!({"n":9,"list":[4]}));
    }

    #[test]
    fn update_object_local_add_then_patch_merges() {
        let local = Arc::new(RwLock::new(HashMap::new()));
        update_object_local(&local, event("ADD_ITEM", Some("doc"), Some(r#"{"a":1,"b":{"c":2}}"#)));
        update_object_local(&local, event("PATCH_ITEM", Some("doc"), Some(r#"{"b":{"d":3}}"#)));
        assert_eq!(local.read().unwrap().get("doc").unwrap(), &json!({"a":1,"b":{"c":2,"d":3}}));
    }

    #[test]
    fn update_object_local_patch_on_absent_key_starts_from_delta() {
        let local = Arc::new(RwLock::new(HashMap::new()));
        update_object_local(&local, event("PATCH_ITEM", Some("doc"), Some(r#"{"a":1}"#)));
        assert_eq!(local.read().unwrap().get("doc").unwrap(), &json!({"a":1}));
    }

    #[test]
    fn update_object_local_remove_and_clear() {
        let local = Arc::new(RwLock::new(HashMap::new()));
        update_object_local(&local, event("ADD_ITEM", Some("doc"), Some(r#"{"a":1}"#)));
        update_object_local(&local, event("REMOVE_ITEM", Some("doc"), None));
        assert!(local.read().unwrap().get("doc").is_none());
        update_object_local(&local, event("ADD_ITEM", Some("doc"), Some(r#"{"a":1}"#)));
        update_object_local(&local, event("CLEAR_CACHE", None, None));
        assert!(local.read().unwrap().is_empty());
    }

    #[test]
    fn update_cache_local_ignores_patch_item() {
        let local = Arc::new(RwLock::new(HashMap::new()));
        update_cache_local(&local, event("ADD_ITEM", Some("doc"), Some(r#"{"a":1,"b":2}"#)));
        // A stray PATCH_ITEM delta must NOT clobber the full stored value in the string mirror.
        update_cache_local(&local, event("PATCH_ITEM", Some("doc"), Some(r#"{"b":3}"#)));
        assert_eq!(local.read().unwrap().get("doc").unwrap(), r#"{"a":1,"b":2}"#);
    }
}
