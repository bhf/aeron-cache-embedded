//! Bidirectional WebSocket transport for Aeron Cache — the JSON/WebSocket analogue of the Aeron
//! [`crate::AeronGatewayClient`], and an alternative to the HTTP+WS [`crate::AeronCacheClient`].
//!
//! A single WebSocket connection to `/api/ws/v1/bidi` carries the full cache + counter command
//! surface plus dynamic subscribe/unsubscribe, multiplexed by a client-minted `correlationId` that
//! the server echoes on every correlated frame. Frames are JSON text messages tagged with a `type`
//! discriminator. A background reader thread drains the socket and dispatches decoded frames to the
//! pending request waiting on them (correlated by correlation id, mirroring
//! [`crate::gateway`]) and to streaming-subscription listeners.
//!
//! ## Stream-update routing
//!
//! Unlike single-response commands, `streamUpdate` frames are stamped with the *causing command's*
//! correlation id (the update's `requestId`), **not** the subscription's id. Stream updates are
//! therefore routed to listeners **by `cacheId`** (a `cacheId -> listeners` map), exactly like the
//! Aeron gateway transport and the Python reference client.

use crate::{
    BulkCacheOpsRequest, BulkCacheOpsResponse, CacheItem, CacheOperationResponse, CacheUpdateEvent,
    CancelItemRemovalResponse, ClearCacheResponse, CounterItem, CounterResponse, CounterUpdateEvent,
    CreateResponse, DeleteCacheResponse, DeleteItemResponse, GetCacheResponse, GetCountersResponse,
    GetItemResponse, PatchItemResponse, PutItemResponse, TimerInfo,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::error::Error;
use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};
use url::Url;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a reader `read` blocks before returning so writers can take the socket lock.
const READ_POLL_TIMEOUT: Duration = Duration::from_millis(50);

type Ws = WebSocket<MaybeTlsStream<TcpStream>>;

// ------------------------------------------------------------------ public types

/// A single cache's stats, as carried in a bidi `stats` frame. Unlike the HTTP aggregate
/// [`crate::CacheStatsResponse`], `getStats` over the bidi transport returns one of these per cache.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatEntry {
    pub cache_id: String,
    #[serde(default)]
    pub added_count: i64,
    #[serde(default)]
    pub removed_count: i64,
    #[serde(default)]
    pub cleared_count: i64,
    #[serde(default)]
    pub size: i64,
}

type CacheListener = Arc<dyn Fn(CacheUpdateEvent) + Send + Sync>;
type CounterListener = Arc<dyn Fn(CounterUpdateEvent) + Send + Sync>;

struct CacheSub {
    cid: String,
    listener: CacheListener,
}

struct CounterSub {
    cid: String,
    listener: CounterListener,
}

/// Data delivered to a waiting batched-entries request once `endOfBatch` arrives.
struct EntriesData {
    items: Map<String, Value>,
    cache_id: String,
    status: String,
}

struct EntriesReq {
    sender: Sender<Result<EntriesData, String>>,
    items: Map<String, Value>,
    cache_id: String,
    status: String,
}

struct StatsReq {
    sender: Sender<Result<Vec<StatEntry>, String>>,
    stats: Vec<StatEntry>,
}

struct TimersReq {
    sender: Sender<Result<Vec<TimerInfo>, String>>,
    timers: Vec<TimerInfo>,
}

struct BulkReq {
    sender: Sender<Result<Vec<CacheOperationResponse>, String>>,
    responses: Vec<CacheOperationResponse>,
}

/// State shared between the background reader thread and the client's command methods.
struct Shared {
    /// correlationId -> waiter for a single `commandResponse`.
    pending_cmd: Mutex<HashMap<String, Sender<Result<Value, String>>>>,
    /// correlationId -> batched `entries` accumulator.
    pending_entries: Mutex<HashMap<String, EntriesReq>>,
    /// correlationId -> batched `stats` accumulator.
    pending_stats: Mutex<HashMap<String, StatsReq>>,
    /// correlationId -> batched `timers` accumulator.
    pending_timers: Mutex<HashMap<String, TimersReq>>,
    /// correlationId -> batched `bulkResponse` accumulator.
    pending_bulk: Mutex<HashMap<String, BulkReq>>,
    /// correlationId -> `subscribed` ack barrier.
    pending_sub_ack: Mutex<HashMap<String, Sender<()>>>,
    /// cacheId -> cache-stream listeners (routed by cacheId, see module docs).
    cache_listeners: Mutex<HashMap<String, Vec<CacheSub>>>,
    /// cacheId -> counter-stream listeners.
    counter_listeners: Mutex<HashMap<String, Vec<CounterSub>>>,
}

impl Shared {
    fn new() -> Self {
        Shared {
            pending_cmd: Mutex::new(HashMap::new()),
            pending_entries: Mutex::new(HashMap::new()),
            pending_stats: Mutex::new(HashMap::new()),
            pending_timers: Mutex::new(HashMap::new()),
            pending_bulk: Mutex::new(HashMap::new()),
            pending_sub_ack: Mutex::new(HashMap::new()),
            cache_listeners: Mutex::new(HashMap::new()),
            counter_listeners: Mutex::new(HashMap::new()),
        }
    }

    fn fail_all(&self, msg: &str) {
        for (_, tx) in self.pending_cmd.lock().unwrap().drain() {
            let _ = tx.send(Err(msg.to_string()));
        }
        for (_, req) in self.pending_entries.lock().unwrap().drain() {
            let _ = req.sender.send(Err(msg.to_string()));
        }
        for (_, req) in self.pending_stats.lock().unwrap().drain() {
            let _ = req.sender.send(Err(msg.to_string()));
        }
        for (_, req) in self.pending_timers.lock().unwrap().drain() {
            let _ = req.sender.send(Err(msg.to_string()));
        }
        for (_, req) in self.pending_bulk.lock().unwrap().drain() {
            let _ = req.sender.send(Err(msg.to_string()));
        }
        self.pending_sub_ack.lock().unwrap().clear();
    }
}

/// A handle to an active bidi subscription. Dropping it (or calling [`BidiSubscription::close`])
/// removes the local listener and best-effort unsubscribes on the server.
pub struct BidiSubscription {
    correlation_id: String,
    cache_id: String,
    counters: bool,
    ws: Arc<Mutex<Ws>>,
    shared: Arc<Shared>,
    closed: bool,
}

impl BidiSubscription {
    /// Explicitly unsubscribe, returning any error from sending the unsubscribe frame.
    pub fn close(mut self) -> Result<(), Box<dyn Error>> {
        self.do_close()
    }

    fn do_close(&mut self) -> Result<(), Box<dyn Error>> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        // Remove the local listener for this subscription's correlation id.
        if self.counters {
            let mut map = self.shared.counter_listeners.lock().unwrap();
            if let Some(list) = map.get_mut(&self.cache_id) {
                list.retain(|s| s.cid != self.correlation_id);
                if list.is_empty() {
                    map.remove(&self.cache_id);
                }
            }
        } else {
            let mut map = self.shared.cache_listeners.lock().unwrap();
            if let Some(list) = map.get_mut(&self.cache_id) {
                list.retain(|s| s.cid != self.correlation_id);
                if list.is_empty() {
                    map.remove(&self.cache_id);
                }
            }
        }
        // Best-effort unsubscribe on the server.
        let frame = json!({
            "type": "unsubscribe",
            "correlationId": new_correlation_id(),
            "counters": self.counters,
            "cacheId": self.cache_id,
        });
        send_frame(&self.ws, &frame)
    }
}

impl Drop for BidiSubscription {
    fn drop(&mut self) {
        let _ = self.do_close();
    }
}

/// High-level client for the Aeron Cache bidirectional WebSocket transport.
pub struct AeronBidiClient {
    ws: Arc<Mutex<Ws>>,
    shared: Arc<Shared>,
    request_timeout: Duration,
    running: Arc<AtomicBool>,
    reader_thread: Option<JoinHandle<()>>,
}

impl AeronBidiClient {
    /// Connect to the bidi endpoint at `{ws_url}/api/ws/v1/bidi`.
    pub fn connect(ws_url: &str) -> Result<Self, Box<dyn Error>> {
        Self::connect_with_timeout(ws_url, DEFAULT_REQUEST_TIMEOUT)
    }

    /// Connect with an explicit per-request timeout.
    pub fn connect_with_timeout(ws_url: &str, request_timeout: Duration) -> Result<Self, Box<dyn Error>> {
        let endpoint = format!("{}/api/ws/v1/bidi", ws_url.trim_end_matches('/'));
        let (mut socket, _resp) = connect(Url::parse(&endpoint)?)?;

        // Give the reader a bounded blocking read so it periodically releases the socket lock for
        // writers (single WebSocket does both read and write in tungstenite).
        set_read_timeout(&mut socket, Some(READ_POLL_TIMEOUT));

        let ws = Arc::new(Mutex::new(socket));
        let shared = Arc::new(Shared::new());
        let running = Arc::new(AtomicBool::new(true));

        let reader_thread = {
            let ws = ws.clone();
            let shared = shared.clone();
            let running = running.clone();
            std::thread::spawn(move || read_loop(ws, shared, running))
        };

        Ok(AeronBidiClient {
            ws,
            shared,
            request_timeout,
            running,
            reader_thread: Some(reader_thread),
        })
    }

    // ------------------------------------------------------------------ cache commands

    pub fn create_cache(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>> {
        let r = self.command("CREATE_CACHE", Some(cache_id), None, None, 0, 0)?;
        Ok(CreateResponse { cache_id: sv(&r, "cacheId"), operation_status: sv(&r, "status") })
    }

    pub fn put_item(&self, cache_id: &str, key: &str, value: &str) -> Result<PutItemResponse, Box<dyn Error>> {
        self.put_timed_item(cache_id, key, value, 0)
    }

    pub fn put_timed_item(&self, cache_id: &str, key: &str, value: &str, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let r = self.command("ADD_CACHE_ENTRY", Some(cache_id), Some(key), Some(value), ttl, 0)?;
        Ok(put_item_response(&r))
    }

    /// Deep-merge `value` into an existing item (PATCH).
    pub fn patch_item(&self, cache_id: &str, key: &str, value: &str) -> Result<PatchItemResponse, Box<dyn Error>> {
        let r = self.command("PATCH_CACHE_ENTRY", Some(cache_id), Some(key), Some(value), 0, 0)?;
        Ok(PatchItemResponse { cache_id: sv(&r, "cacheId"), key: sv(&r, "key"), operation_status: sv(&r, "status") })
    }

    pub fn get_item(&self, cache_id: &str, key: &str) -> Result<GetItemResponse, Box<dyn Error>> {
        let r = self.command("GET_CACHE_ENTRY", Some(cache_id), Some(key), None, 0, 0)?;
        Ok(GetItemResponse {
            cache_id: sv(&r, "cacheId"),
            key: sv(&r, "key"),
            value: sv(&r, "value"),
            operation_status: sv(&r, "status"),
        })
    }

    pub fn delete_item(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        let r = self.command("REMOVE_CACHE_ENTRY", Some(cache_id), Some(key), None, 0, 0)?;
        Ok(delete_item_response(&r))
    }

    /// Cancel a scheduled TTL removal of an item.
    pub fn cancel_item_removal(&self, cache_id: &str, key: &str) -> Result<CancelItemRemovalResponse, Box<dyn Error>> {
        let r = self.command("CANCEL_CACHE_ITEM_REMOVAL", Some(cache_id), Some(key), None, 0, 0)?;
        Ok(CancelItemRemovalResponse { cache_id: sv(&r, "cacheId"), key: sv(&r, "key"), operation_status: sv(&r, "status") })
    }

    pub fn clear_cache(&self, cache_id: &str) -> Result<ClearCacheResponse, Box<dyn Error>> {
        let r = self.command("CLEAR_CACHE", Some(cache_id), None, None, 0, 0)?;
        Ok(ClearCacheResponse { cache_id: sv(&r, "cacheId"), operation_status: sv(&r, "status") })
    }

    pub fn delete_cache(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        let r = self.command("DELETE_CACHE", Some(cache_id), None, None, 0, 0)?;
        Ok(DeleteCacheResponse { cache_id: sv(&r, "cacheId"), operation_status: sv(&r, "status") })
    }

    /// Full snapshot of a cache's entries (streamed as one or more batches).
    pub fn get_cache_items(&self, cache_id: &str) -> Result<GetCacheResponse, Box<dyn Error>> {
        let data = self.batched_entries("GET_CACHE_ENTRIES", Some(cache_id))?;
        let items = data
            .items
            .into_iter()
            .map(|(k, v)| CacheItem { key: k, value: value_to_string(&v) })
            .collect();
        Ok(GetCacheResponse { cache_id: data.cache_id, operation_status: data.status, items })
    }

    /// Per-cache stats (one [`StatEntry`] per cache), streamed as one or more batches.
    pub fn get_stats(&self) -> Result<Vec<StatEntry>, Box<dyn Error>> {
        self.batched_stats("GET_CACHE_STATS")
    }

    // ------------------------------------------------------------------ counter commands

    pub fn create_counter_cache(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>> {
        let r = self.command("CREATE_COUNTER_CACHE", Some(cache_id), None, None, 0, 0)?;
        Ok(CreateResponse { cache_id: sv(&r, "cacheId"), operation_status: sv(&r, "status") })
    }

    pub fn put_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let r = self.command("ADD_COUNTER_ENTRY", Some(cache_id), Some(key), None, 0, value)?;
        Ok(put_item_response(&r))
    }

    pub fn put_timed_counter(&self, cache_id: &str, key: &str, value: i64, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let r = self.command("ADD_COUNTER_ENTRY", Some(cache_id), Some(key), None, ttl, value)?;
        Ok(put_item_response(&r))
    }

    pub fn get_counter(&self, cache_id: &str, key: &str) -> Result<CounterResponse, Box<dyn Error>> {
        let r = self.command("GET_COUNTER_ENTRY", Some(cache_id), Some(key), None, 0, 0)?;
        Ok(counter_response(&r))
    }

    pub fn delete_counter(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        let r = self.command("REMOVE_COUNTER_ENTRY", Some(cache_id), Some(key), None, 0, 0)?;
        Ok(delete_item_response(&r))
    }

    /// Cancel a scheduled TTL removal of a counter.
    pub fn cancel_counter_item_removal(&self, cache_id: &str, key: &str) -> Result<CancelItemRemovalResponse, Box<dyn Error>> {
        let r = self.command("CANCEL_COUNTER_ITEM_REMOVAL", Some(cache_id), Some(key), None, 0, 0)?;
        Ok(CancelItemRemovalResponse { cache_id: sv(&r, "cacheId"), key: sv(&r, "key"), operation_status: sv(&r, "status") })
    }

    pub fn clear_counter_cache(&self, cache_id: &str) -> Result<ClearCacheResponse, Box<dyn Error>> {
        let r = self.command("CLEAR_COUNTER_CACHE", Some(cache_id), None, None, 0, 0)?;
        Ok(ClearCacheResponse { cache_id: sv(&r, "cacheId"), operation_status: sv(&r, "status") })
    }

    pub fn delete_counter_cache(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        let r = self.command("DELETE_COUNTER_CACHE", Some(cache_id), None, None, 0, 0)?;
        Ok(DeleteCacheResponse { cache_id: sv(&r, "cacheId"), operation_status: sv(&r, "status") })
    }

    pub fn increment_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let r = self.command("INCREMENT_COUNTER_ENTRY", Some(cache_id), Some(key), None, 0, amount)?;
        Ok(counter_response(&r))
    }

    pub fn decrement_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let r = self.command("DECREMENT_COUNTER_ENTRY", Some(cache_id), Some(key), None, 0, amount)?;
        Ok(counter_response(&r))
    }

    pub fn set_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let r = self.command("SET_COUNTER_ENTRY", Some(cache_id), Some(key), None, 0, value)?;
        Ok(counter_response(&r))
    }

    /// Full snapshot of a counter cache's entries (values parsed to `i64`).
    pub fn get_counter_items(&self, cache_id: &str) -> Result<GetCountersResponse, Box<dyn Error>> {
        let data = self.batched_entries("GET_COUNTER_ENTRIES", Some(cache_id))?;
        let items = data
            .items
            .into_iter()
            .map(|(k, v)| CounterItem { key: k, value: value_to_i64(&v) })
            .collect();
        Ok(GetCountersResponse { cache_id: data.cache_id, operation_status: data.status, items })
    }

    /// Per-counter-cache stats (one [`StatEntry`] per counter cache).
    pub fn get_counter_stats(&self) -> Result<Vec<StatEntry>, Box<dyn Error>> {
        self.batched_stats("GET_COUNTER_STATS")
    }

    // ------------------------------------------------------------------ timers

    /// All pending TTL removal timers across both caches and counter caches. The server streams one or
    /// more `timers` frames, accumulated until the end-of-batch frame; each timer is tagged `CACHE` or
    /// `COUNTER`.
    pub fn get_timers(&self) -> Result<Vec<TimerInfo>, Box<dyn Error>> {
        let cid = new_correlation_id();
        let (tx, rx) = channel();
        self.shared.pending_timers.lock().unwrap().insert(cid.clone(), TimersReq { sender: tx, timers: Vec::new() });
        let frame = json!({
            "type": "command",
            "correlationId": cid,
            "op": "GET_TIMERS",
            "cacheId": Value::Null,
            "key": Value::Null,
            "value": Value::Null,
            "ttl": 0,
            "counterValue": 0,
        });
        if let Err(e) = send_frame(&self.ws, &frame) {
            self.shared.pending_timers.lock().unwrap().remove(&cid);
            return Err(e);
        }
        match rx.recv_timeout(self.request_timeout) {
            Ok(Ok(timers)) => Ok(timers),
            Ok(Err(msg)) => Err(msg.into()),
            Err(_) => {
                self.shared.pending_timers.lock().unwrap().remove(&cid);
                Err("bidi request timed out".into())
            }
        }
    }

    // ------------------------------------------------------------------ bulk operations

    /// Apply a batch of cache/counter operations in one `bulk` frame, mirroring the HTTP client's
    /// [`crate::AeronCacheClient::bulk_ops`]. A batch may freely mix regular-cache and counter
    /// operations, applied in request order. Per-operation results are streamed back as one or more
    /// `bulkResponse` frames, accumulated until the end-of-batch frame; each echoes its `requestId`.
    pub fn bulk_ops(&self, req: &BulkCacheOpsRequest) -> Result<BulkCacheOpsResponse, Box<dyn Error>> {
        let cid = if req.request_id.is_empty() { new_correlation_id() } else { req.request_id.clone() };
        let (tx, rx) = channel();
        self.shared.pending_bulk.lock().unwrap().insert(cid.clone(), BulkReq { sender: tx, responses: Vec::new() });
        let operations = serde_json::to_value(&req.operations)?;
        let frame = json!({
            "type": "bulk",
            "correlationId": cid,
            "operations": operations,
        });
        if let Err(e) = send_frame(&self.ws, &frame) {
            self.shared.pending_bulk.lock().unwrap().remove(&cid);
            return Err(e);
        }
        match rx.recv_timeout(self.request_timeout) {
            Ok(Ok(responses)) => Ok(BulkCacheOpsResponse { request_id: cid, operation_responses: responses }),
            Ok(Err(msg)) => Err(msg.into()),
            Err(_) => {
                self.shared.pending_bulk.lock().unwrap().remove(&cid);
                Err("bidi request timed out".into())
            }
        }
    }

    // ------------------------------------------------------------------ subscriptions

    /// Subscribe to streaming updates for a cache. `listener` is invoked on the reader thread.
    pub fn subscribe<F>(&self, cache_id: &str, listener: F) -> Result<BidiSubscription, Box<dyn Error>>
    where
        F: Fn(CacheUpdateEvent) + Send + Sync + 'static,
    {
        self.subscribe_with(cache_id, false, None, None, listener)
    }

    /// Subscribe to a cache with an optional snapshot, key filter, and subscription `mode`
    /// (`"full"` / `"patch"`; sent uppercased). Waits best-effort up to the request timeout for the
    /// server's `subscribed` ack, then proceeds regardless.
    pub fn subscribe_with<F>(&self, cache_id: &str, send_snapshot: bool, key: Option<&str>, mode: Option<&str>, listener: F) -> Result<BidiSubscription, Box<dyn Error>>
    where
        F: Fn(CacheUpdateEvent) + Send + Sync + 'static,
    {
        let cid = new_correlation_id();
        self.shared
            .cache_listeners
            .lock()
            .unwrap()
            .entry(cache_id.to_string())
            .or_default()
            .push(CacheSub { cid: cid.clone(), listener: Arc::new(listener) });
        self.send_subscribe(&cid, cache_id, false, send_snapshot, key, mode)?;
        Ok(BidiSubscription {
            correlation_id: cid,
            cache_id: cache_id.to_string(),
            counters: false,
            ws: self.ws.clone(),
            shared: self.shared.clone(),
            closed: false,
        })
    }

    /// Subscribe to streaming updates for a counter cache.
    pub fn subscribe_counter<F>(&self, cache_id: &str, listener: F) -> Result<BidiSubscription, Box<dyn Error>>
    where
        F: Fn(CounterUpdateEvent) + Send + Sync + 'static,
    {
        self.subscribe_counter_with(cache_id, false, None, listener)
    }

    /// Subscribe to a counter cache with an optional snapshot and key filter. (Patch mode is
    /// cache-only and does not apply to counter subscriptions.)
    pub fn subscribe_counter_with<F>(&self, cache_id: &str, send_snapshot: bool, key: Option<&str>, listener: F) -> Result<BidiSubscription, Box<dyn Error>>
    where
        F: Fn(CounterUpdateEvent) + Send + Sync + 'static,
    {
        let cid = new_correlation_id();
        self.shared
            .counter_listeners
            .lock()
            .unwrap()
            .entry(cache_id.to_string())
            .or_default()
            .push(CounterSub { cid: cid.clone(), listener: Arc::new(listener) });
        self.send_subscribe(&cid, cache_id, true, send_snapshot, key, None)?;
        Ok(BidiSubscription {
            correlation_id: cid,
            cache_id: cache_id.to_string(),
            counters: true,
            ws: self.ws.clone(),
            shared: self.shared.clone(),
            closed: false,
        })
    }

    // ------------------------------------------------------------------ internals

    fn command(&self, op: &str, cache_id: Option<&str>, key: Option<&str>, value: Option<&str>, ttl: i64, counter_value: i64) -> Result<Value, Box<dyn Error>> {
        let cid = new_correlation_id();
        let (tx, rx) = channel();
        self.shared.pending_cmd.lock().unwrap().insert(cid.clone(), tx);
        let frame = json!({
            "type": "command",
            "correlationId": cid,
            "op": op,
            "cacheId": cache_id,
            "key": key,
            "value": value,
            "ttl": ttl,
            "counterValue": counter_value,
        });
        if let Err(e) = send_frame(&self.ws, &frame) {
            self.shared.pending_cmd.lock().unwrap().remove(&cid);
            return Err(e);
        }
        match rx.recv_timeout(self.request_timeout) {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(msg)) => Err(msg.into()),
            Err(_) => {
                self.shared.pending_cmd.lock().unwrap().remove(&cid);
                Err("bidi request timed out".into())
            }
        }
    }

    fn batched_entries(&self, op: &str, cache_id: Option<&str>) -> Result<EntriesData, Box<dyn Error>> {
        let cid = new_correlation_id();
        let (tx, rx) = channel();
        self.shared.pending_entries.lock().unwrap().insert(
            cid.clone(),
            EntriesReq { sender: tx, items: Map::new(), cache_id: String::new(), status: String::new() },
        );
        let frame = json!({
            "type": "command",
            "correlationId": cid,
            "op": op,
            "cacheId": cache_id,
            "key": Value::Null,
            "value": Value::Null,
            "ttl": 0,
            "counterValue": 0,
        });
        if let Err(e) = send_frame(&self.ws, &frame) {
            self.shared.pending_entries.lock().unwrap().remove(&cid);
            return Err(e);
        }
        match rx.recv_timeout(self.request_timeout) {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(msg)) => Err(msg.into()),
            Err(_) => {
                self.shared.pending_entries.lock().unwrap().remove(&cid);
                Err("bidi request timed out".into())
            }
        }
    }

    fn batched_stats(&self, op: &str) -> Result<Vec<StatEntry>, Box<dyn Error>> {
        let cid = new_correlation_id();
        let (tx, rx) = channel();
        self.shared.pending_stats.lock().unwrap().insert(cid.clone(), StatsReq { sender: tx, stats: Vec::new() });
        let frame = json!({
            "type": "command",
            "correlationId": cid,
            "op": op,
            "cacheId": Value::Null,
            "key": Value::Null,
            "value": Value::Null,
            "ttl": 0,
            "counterValue": 0,
        });
        if let Err(e) = send_frame(&self.ws, &frame) {
            self.shared.pending_stats.lock().unwrap().remove(&cid);
            return Err(e);
        }
        match rx.recv_timeout(self.request_timeout) {
            Ok(Ok(stats)) => Ok(stats),
            Ok(Err(msg)) => Err(msg.into()),
            Err(_) => {
                self.shared.pending_stats.lock().unwrap().remove(&cid);
                Err("bidi request timed out".into())
            }
        }
    }

    fn send_subscribe(&self, cid: &str, cache_id: &str, counters: bool, send_snapshot: bool, key: Option<&str>, mode: Option<&str>) -> Result<(), Box<dyn Error>> {
        let mut selector = Map::new();
        selector.insert("cacheId".to_string(), Value::String(cache_id.to_string()));
        if let Some(k) = key {
            selector.insert("key".to_string(), Value::String(k.to_string()));
        }
        if let Some(m) = mode {
            selector.insert("mode".to_string(), Value::String(m.to_uppercase()));
        }
        let (tx, rx) = channel();
        self.shared.pending_sub_ack.lock().unwrap().insert(cid.to_string(), tx);
        let frame = json!({
            "type": "subscribe",
            "correlationId": cid,
            "counters": counters,
            "sendSnapshot": send_snapshot,
            "caches": [Value::Object(selector)],
        });
        if let Err(e) = send_frame(&self.ws, &frame) {
            self.shared.pending_sub_ack.lock().unwrap().remove(cid);
            return Err(e);
        }
        // Best-effort barrier: proceed regardless of whether the ack arrives in time.
        let _ = rx.recv_timeout(self.request_timeout);
        self.shared.pending_sub_ack.lock().unwrap().remove(cid);
        Ok(())
    }
}

impl Drop for AeronBidiClient {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        // Close the socket so a blocked/looping reader observes end-of-stream and exits.
        if let Ok(mut ws) = self.ws.lock() {
            let _ = ws.close(None);
        }
        self.shared.fail_all("AeronBidiClient closed");
        if let Some(handle) = self.reader_thread.take() {
            let _ = handle.join();
        }
    }
}

// ------------------------------------------------------------------ reader / dispatch

fn read_loop(ws: Arc<Mutex<Ws>>, shared: Arc<Shared>, running: Arc<AtomicBool>) {
    while running.load(Ordering::Acquire) {
        let msg = {
            let mut sock = ws.lock().unwrap();
            sock.read_message()
        };
        match msg {
            Ok(Message::Text(txt)) => {
                if let Ok(value) = serde_json::from_str::<Value>(&txt) {
                    dispatch(&value, &shared);
                }
            }
            Ok(Message::Binary(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Frame(_)) => {}
            Ok(Message::Close(_)) => {
                shared.fail_all("bidi connection closed");
                break;
            }
            Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                // Read poll expired; release the lock (via drop above) and let writers run.
            }
            Err(tungstenite::Error::ConnectionClosed) | Err(tungstenite::Error::AlreadyClosed) => {
                shared.fail_all("bidi connection closed");
                break;
            }
            Err(_) => {
                // Transient error; back off briefly and retry.
            }
        }
        // Yield so command threads can take the socket lock to write.
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn dispatch(msg: &Value, shared: &Shared) {
    let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let cid = msg.get("correlationId").and_then(|v| v.as_str()).unwrap_or("").to_string();
    match msg_type {
        "commandResponse" => {
            if let Some(tx) = shared.pending_cmd.lock().unwrap().remove(&cid) {
                let _ = tx.send(Ok(msg.clone()));
            }
        }
        "entries" => {
            let end_of_batch = msg.get("endOfBatch").and_then(|v| v.as_bool()).unwrap_or(false);
            let mut map = shared.pending_entries.lock().unwrap();
            if let Some(req) = map.get_mut(&cid) {
                if let Some(items) = msg.get("items").and_then(|v| v.as_object()) {
                    for (k, v) in items {
                        req.items.insert(k.clone(), v.clone());
                    }
                }
                req.cache_id = msg.get("cacheId").and_then(|v| v.as_str()).unwrap_or("").to_string();
                req.status = msg.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if end_of_batch {
                    let req = map.remove(&cid).unwrap();
                    let _ = req.sender.send(Ok(EntriesData { items: req.items, cache_id: req.cache_id, status: req.status }));
                }
            }
        }
        "stats" => {
            let end_of_batch = msg.get("endOfBatch").and_then(|v| v.as_bool()).unwrap_or(false);
            let mut map = shared.pending_stats.lock().unwrap();
            if let Some(req) = map.get_mut(&cid) {
                if let Some(arr) = msg.get("stats").and_then(|v| v.as_array()) {
                    for s in arr {
                        if let Ok(entry) = serde_json::from_value::<StatEntry>(s.clone()) {
                            req.stats.push(entry);
                        }
                    }
                }
                if end_of_batch {
                    let req = map.remove(&cid).unwrap();
                    let _ = req.sender.send(Ok(req.stats));
                }
            }
        }
        "timers" => {
            let end_of_batch = msg.get("endOfBatch").and_then(|v| v.as_bool()).unwrap_or(false);
            let mut map = shared.pending_timers.lock().unwrap();
            if let Some(req) = map.get_mut(&cid) {
                if let Some(arr) = msg.get("timers").and_then(|v| v.as_array()) {
                    for t in arr {
                        if let Ok(timer) = serde_json::from_value::<TimerInfo>(t.clone()) {
                            req.timers.push(timer);
                        }
                    }
                }
                if end_of_batch {
                    let req = map.remove(&cid).unwrap();
                    let _ = req.sender.send(Ok(req.timers));
                }
            }
        }
        "bulkResponse" => {
            let end_of_batch = msg.get("endOfBatch").and_then(|v| v.as_bool()).unwrap_or(false);
            let mut map = shared.pending_bulk.lock().unwrap();
            if let Some(req) = map.get_mut(&cid) {
                if let Some(arr) = msg.get("operationResponses").and_then(|v| v.as_array()) {
                    for o in arr {
                        if let Ok(resp) = serde_json::from_value::<CacheOperationResponse>(o.clone()) {
                            req.responses.push(resp);
                        }
                    }
                }
                if end_of_batch {
                    let req = map.remove(&cid).unwrap();
                    let _ = req.sender.send(Ok(req.responses));
                }
            }
        }
        "subscribed" => {
            if let Some(tx) = shared.pending_sub_ack.lock().unwrap().remove(&cid) {
                let _ = tx.send(());
            }
        }
        "streamUpdate" => dispatch_stream_update(&cid, msg, shared),
        "error" => dispatch_error(&cid, msg, shared),
        _ => {}
    }
}

fn dispatch_stream_update(cid: &str, msg: &Value, shared: &Shared) {
    let cache_id = msg.get("cacheId").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let event_type = msg.get("eventType").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let key = msg.get("key").and_then(|v| v.as_str()).map(|s| s.to_string());
    let value = msg.get("value").cloned().unwrap_or(Value::Null);

    {
        let listeners = shared.cache_listeners.lock().unwrap();
        if let Some(list) = listeners.get(&cache_id) {
            let event = CacheUpdateEvent {
                cache_id: cache_id.clone(),
                event_type: event_type.clone(),
                item_key: key.clone(),
                item_value: if value.is_null() { None } else { Some(value_to_string(&value)) },
                request_id: cid.to_string(),
            };
            for s in list.iter() {
                (s.listener)(event.clone());
            }
        }
    }
    {
        let listeners = shared.counter_listeners.lock().unwrap();
        if let Some(list) = listeners.get(&cache_id) {
            let event = CounterUpdateEvent {
                cache_id: cache_id.clone(),
                event_type: event_type.clone(),
                item_key: key.clone(),
                item_value: if value.is_null() { None } else { Some(value_to_i64(&value)) },
                request_id: cid.to_string(),
            };
            for s in list.iter() {
                (s.listener)(event.clone());
            }
        }
    }
}

fn dispatch_error(cid: &str, msg: &Value, shared: &Shared) {
    let status = msg.get("status").and_then(|v| v.as_str()).unwrap_or("ERROR");
    let message = msg.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let err = format!("bidi error [{status}]: {message}");
    if cid.is_empty() {
        return;
    }
    if let Some(tx) = shared.pending_cmd.lock().unwrap().remove(cid) {
        let _ = tx.send(Err(err));
        return;
    }
    if let Some(req) = shared.pending_entries.lock().unwrap().remove(cid) {
        let _ = req.sender.send(Err(err));
        return;
    }
    if let Some(req) = shared.pending_stats.lock().unwrap().remove(cid) {
        let _ = req.sender.send(Err(err));
        return;
    }
    if let Some(req) = shared.pending_timers.lock().unwrap().remove(cid) {
        let _ = req.sender.send(Err(err));
        return;
    }
    if let Some(req) = shared.pending_bulk.lock().unwrap().remove(cid) {
        let _ = req.sender.send(Err(err));
        return;
    }
    shared.pending_sub_ack.lock().unwrap().remove(cid);
}

// ------------------------------------------------------------------ helpers

fn new_correlation_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}-{n:x}")
}

fn send_frame(ws: &Arc<Mutex<Ws>>, frame: &Value) -> Result<(), Box<dyn Error>> {
    let text = serde_json::to_string(frame)?;
    let mut sock = ws.lock().unwrap();
    sock.write_message(Message::Text(text))?;
    // Flush the send queue (also drains any queued pong replies).
    match sock.write_pending() {
        Ok(()) => Ok(()),
        Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Set a read timeout on the underlying TCP stream so the reader periodically releases the socket
/// lock for writers. Best-effort: only the plain (`ws://`) stream is handled; the bidi endpoint is
/// used over `ws://`.
fn set_read_timeout(ws: &mut Ws, timeout: Option<Duration>) {
    if let MaybeTlsStream::Plain(s) = ws.get_mut() {
        let _ = s.set_read_timeout(timeout);
    }
}

/// Render a JSON value as a plain string (strings unquoted; numbers/bools stringified).
fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Parse a JSON value as an `i64` (numbers directly; numeric strings parsed; else 0).
fn value_to_i64(v: &Value) -> i64 {
    match v {
        Value::Number(n) => n.as_i64().unwrap_or_else(|| n.as_f64().map(|f| f as i64).unwrap_or(0)),
        Value::String(s) => s.parse::<i64>().unwrap_or(0),
        _ => 0,
    }
}

/// Extract a string field from a JSON object (missing/null -> empty string).
fn sv(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn put_item_response(r: &Value) -> PutItemResponse {
    let status = sv(r, "status");
    PutItemResponse {
        cache_id: sv(r, "cacheId"),
        key: sv(r, "key"),
        status: status.clone(),
        operation_status: status,
    }
}

fn delete_item_response(r: &Value) -> DeleteItemResponse {
    DeleteItemResponse { cache_id: sv(r, "cacheId"), key: sv(r, "key"), operation_status: sv(r, "status") }
}

fn counter_response(r: &Value) -> CounterResponse {
    let value = r.get("value").map(value_to_i64).unwrap_or(0);
    CounterResponse {
        cache_id: sv(r, "cacheId"),
        key: sv(r, "key"),
        value,
        operation_status: sv(r, "status"),
    }
}

// ------------------------------------------------------------------ unit tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_to_string_unquotes_strings_and_stringifies_others() {
        assert_eq!(value_to_string(&json!("hello")), "hello");
        assert_eq!(value_to_string(&json!(42)), "42");
        assert_eq!(value_to_string(&Value::Null), "");
    }

    #[test]
    fn value_to_i64_parses_numbers_and_numeric_strings() {
        assert_eq!(value_to_i64(&json!(15)), 15);
        assert_eq!(value_to_i64(&json!("100")), 100);
        assert_eq!(value_to_i64(&json!("nope")), 0);
        assert_eq!(value_to_i64(&Value::Null), 0);
    }

    #[test]
    fn command_response_maps_status_and_fields() {
        let r = json!({"cacheId": "c1", "key": "k1", "value": "v1", "status": "SUCCESS"});
        let put = put_item_response(&r);
        assert_eq!(put.cache_id, "c1");
        assert_eq!(put.key, "k1");
        assert_eq!(put.operation_status, "SUCCESS");
        let ctr = counter_response(&json!({"cacheId": "c1", "key": "hits", "value": 12, "status": "SUCCESS"}));
        assert_eq!(ctr.value, 12);
    }

    #[test]
    fn dispatch_command_response_routes_by_correlation_id() {
        let shared = Shared::new();
        let (tx, rx) = channel();
        shared.pending_cmd.lock().unwrap().insert("abc".to_string(), tx);
        dispatch(&json!({"type": "commandResponse", "correlationId": "abc", "status": "SUCCESS", "cacheId": "c1"}), &shared);
        let v = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        assert_eq!(sv(&v, "status"), "SUCCESS");
    }

    #[test]
    fn entries_accumulate_until_end_of_batch() {
        let shared = Shared::new();
        let (tx, rx) = channel();
        shared.pending_entries.lock().unwrap().insert(
            "e1".to_string(),
            EntriesReq { sender: tx, items: Map::new(), cache_id: String::new(), status: String::new() },
        );
        dispatch(&json!({"type": "entries", "correlationId": "e1", "status": "SUCCESS", "cacheId": "c1", "items": {"a": "1"}, "endOfBatch": false}), &shared);
        assert!(rx.try_recv().is_err(), "must not deliver before endOfBatch");
        dispatch(&json!({"type": "entries", "correlationId": "e1", "status": "SUCCESS", "cacheId": "c1", "items": {"b": "2"}, "endOfBatch": true}), &shared);
        let data = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        assert_eq!(data.cache_id, "c1");
        assert_eq!(data.items.len(), 2);
    }

    #[test]
    fn stats_frame_deserializes_stat_entries() {
        let shared = Shared::new();
        let (tx, rx) = channel();
        shared.pending_stats.lock().unwrap().insert("s1".to_string(), StatsReq { sender: tx, stats: Vec::new() });
        dispatch(&json!({
            "type": "stats", "correlationId": "s1", "status": "SUCCESS", "endOfBatch": true,
            "stats": [{"cacheId": "c1", "addedCount": 3, "removedCount": 1, "clearedCount": 0, "size": 2}]
        }), &shared);
        let stats = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].cache_id, "c1");
        assert_eq!(stats[0].added_count, 3);
        assert_eq!(stats[0].size, 2);
    }

    #[test]
    fn stream_update_routes_to_cache_listener_by_cache_id() {
        let shared = Arc::new(Shared::new());
        let got: Arc<Mutex<Vec<CacheUpdateEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = got.clone();
        shared.cache_listeners.lock().unwrap().entry("c1".to_string()).or_default().push(CacheSub {
            cid: "sub-1".to_string(),
            listener: Arc::new(move |e| sink.lock().unwrap().push(e)),
        });
        // Note: streamUpdate.correlationId is the causing command's id, not the subscription id.
        dispatch(&json!({"type": "streamUpdate", "correlationId": "cmd-99", "eventType": "ADD_ITEM", "cacheId": "c1", "key": "sk", "value": "sv"}), &shared);
        let events = got.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "ADD_ITEM");
        assert_eq!(events[0].item_key.as_deref(), Some("sk"));
        assert_eq!(events[0].item_value.as_deref(), Some("sv"));
        assert_eq!(events[0].request_id, "cmd-99");
    }

    #[test]
    fn stream_update_parses_counter_values_as_numbers() {
        let shared = Arc::new(Shared::new());
        let got: Arc<Mutex<Vec<CounterUpdateEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = got.clone();
        shared.counter_listeners.lock().unwrap().entry("cc".to_string()).or_default().push(CounterSub {
            cid: "sub-2".to_string(),
            listener: Arc::new(move |e| sink.lock().unwrap().push(e)),
        });
        dispatch(&json!({"type": "streamUpdate", "correlationId": "cmd-1", "eventType": "ADD_ITEM", "cacheId": "cc", "key": "hits", "value": 15}), &shared);
        let events = got.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].item_value, Some(15));
    }

    #[test]
    fn error_frame_fails_the_matching_pending_command() {
        let shared = Shared::new();
        let (tx, rx) = channel();
        shared.pending_cmd.lock().unwrap().insert("x".to_string(), tx);
        dispatch(&json!({"type": "error", "correlationId": "x", "status": "BAD_REQUEST", "message": "boom"}), &shared);
        let res = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(res.is_err());
    }
}
