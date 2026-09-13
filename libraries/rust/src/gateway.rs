//! Aeron gateway transport for Aeron Cache — the low-latency, bidirectional alternative to the
//! HTTP+WS [`crate::AeronCacheClient`].
//!
//! It speaks the shared SBE wire protocol (`sbe/gateway-schema.xml`) over Aeron response channels
//! (`control-mode=response`): the client opens a response subscription and stamps its registration id
//! onto the request publication (`response-correlation-id`), so the gateway routes responses back
//! without an application-level handshake. A background thread polls the response subscription and
//! dispatches decoded frames to the pending request that is waiting on them (correlated by a
//! client-minted correlation id) and to streaming-subscription listeners.

use crate::gateway_messages::message_header_codec;
use crate::gateway_messages::{
    boolean_type, bulk_operation_type, gateway_bulk_request_codec, gateway_bulk_response_codec,
    gateway_command_codec, gateway_command_response_codec, gateway_entries_codec,
    gateway_error_codec, gateway_stats_codec, gateway_stream_update_codec, gateway_subscribe_ack_codec,
    gateway_subscribe_codec, gateway_unsubscribe_codec, operation_status, subscription_mode,
    update_event_type, ReadBuf, WriteBuf,
};
use crate::{
    BulkCacheOpsRequest, BulkCacheOpsResponse, CacheItem, CacheOperationResponse, CacheUpdateEvent,
    CounterResponse, CounterUpdateEvent, CreateResponse, DeleteCacheResponse, DeleteItemResponse,
    GetCacheResponse, GetItemResponse, PutItemResponse,
};
use rusteron_client::*;
use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

// ------------------------------------------------------------------ constants

/// Default gateway request endpoint port.
pub const DEFAULT_REQUEST_PORT: u16 = 7075;
/// Default gateway response control endpoint port.
pub const DEFAULT_RESPONSE_CONTROL_PORT: u16 = 7076;
/// Default gateway request stream id.
pub const DEFAULT_REQUEST_STREAM_ID: i32 = 100;
/// Default gateway response stream id.
pub const DEFAULT_RESPONSE_STREAM_ID: i32 = 101;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const FRAGMENT_LIMIT: usize = 16;

/// Message type ids carried in `GatewayCommand.msgType` (mirror the server's `CacheRequestMessageTypes`).
mod msg {
    pub const CREATE_CACHE: u16 = 1;
    pub const ADD_CACHE_ENTRY: u16 = 2;
    pub const GET_CACHE_ENTRY: u16 = 3;
    pub const CLEAR_CACHE: u16 = 4;
    pub const DELETE_CACHE: u16 = 5;
    pub const GET_CACHE_ENTRIES: u16 = 6;
    pub const GET_CACHE_STATS: u16 = 9;
    pub const REMOVE_CACHE_ENTRY: u16 = 10;
    pub const PATCH_CACHE_ENTRY: u16 = 12;
    pub const CANCEL_CACHE_ITEM_REMOVAL: u16 = 13;

    pub const CREATE_COUNTER_CACHE: u16 = 101;
    pub const ADD_COUNTER_ENTRY: u16 = 102;
    pub const GET_COUNTER_ENTRY: u16 = 103;
    pub const DELETE_COUNTER_CACHE: u16 = 105;
    pub const REMOVE_COUNTER_ENTRY: u16 = 110;
    pub const INCREMENT_COUNTER_ENTRY: u16 = 111;
    pub const DECREMENT_COUNTER_ENTRY: u16 = 112;
    pub const SET_COUNTER_ENTRY: u16 = 113;
    pub const CANCEL_COUNTER_ITEM_REMOVAL: u16 = 114;
}

// ------------------------------------------------------------------ public types

/// A single cache's stats, as streamed in a `GatewayStats` frame.
#[derive(Debug, Clone)]
pub struct GatewayStat {
    pub cache_id: String,
    pub added_count: i64,
    pub removed_count: i64,
    pub cleared_count: i64,
    pub size: i64,
}

type CacheListener = Arc<dyn Fn(CacheUpdateEvent) + Send + Sync>;
type CounterListener = Arc<dyn Fn(CounterUpdateEvent) + Send + Sync>;

/// A decoded command response (status + optional entry fields).
#[derive(Debug, Clone)]
struct CommandResponse {
    status: String,
    cache_id: String,
    key: String,
    value: String,
}

/// Outcome delivered to a waiting single-response command.
type CmdOutcome = Result<CommandResponse, String>;

struct EntriesReq {
    sender: Sender<Result<GetCacheResponse, String>>,
    items: Vec<CacheItem>,
    cache_id: String,
    status: String,
}

struct StatsReq {
    sender: Sender<Result<Vec<GatewayStat>, String>>,
    stats: Vec<GatewayStat>,
}

/// State shared between the polling thread and the client's command methods.
struct Shared {
    pending_cmd: Mutex<HashMap<String, Sender<CmdOutcome>>>,
    pending_entries: Mutex<HashMap<String, EntriesReq>>,
    pending_stats: Mutex<HashMap<String, StatsReq>>,
    /// Pending bulk requests, awaiting a reassembled `GatewayBulkResponse`.
    pending_bulk: Mutex<HashMap<String, Sender<Result<BulkCacheOpsResponse, String>>>>,
    /// Pending subscribe barriers, completed by a `GatewaySubscribeAck`.
    pending_sub_ack: Mutex<HashMap<String, Sender<()>>>,
    cache_listeners: Mutex<HashMap<String, Vec<CacheListener>>>,
    counter_listeners: Mutex<HashMap<String, Vec<CounterListener>>>,
    /// Set true once the response subscription has delivered any frame.
    received_any: AtomicBool,
}

impl Shared {
    fn new() -> Self {
        Shared {
            pending_cmd: Mutex::new(HashMap::new()),
            pending_entries: Mutex::new(HashMap::new()),
            pending_stats: Mutex::new(HashMap::new()),
            pending_bulk: Mutex::new(HashMap::new()),
            pending_sub_ack: Mutex::new(HashMap::new()),
            cache_listeners: Mutex::new(HashMap::new()),
            counter_listeners: Mutex::new(HashMap::new()),
            received_any: AtomicBool::new(false),
        }
    }
}

/// A handle to an active gateway subscription. Dropping it unsubscribes.
pub struct GatewaySubscription {
    cache_id: String,
    counters: bool,
    publication: Arc<Mutex<AeronPublication>>,
    shared: Arc<Shared>,
}

impl Drop for GatewaySubscription {
    fn drop(&mut self) {
        // Remove local listeners.
        if self.counters {
            self.shared.counter_listeners.lock().unwrap().remove(&self.cache_id);
        } else {
            self.shared.cache_listeners.lock().unwrap().remove(&self.cache_id);
        }
        // Best-effort unsubscribe to the gateway.
        let frame = encode_unsubscribe(&new_correlation_id(), &self.cache_id, self.counters);
        let _ = offer_frame(&self.publication, &frame, DEFAULT_REQUEST_TIMEOUT);
    }
}

/// High-level client for the Aeron Cache gateway over the Aeron transport.
pub struct AeronGatewayClient {
    // Kept alive for the lifetime of the client (order matters: dropped after the poll thread joins).
    _ctx: AeronContext,
    _aeron: Aeron,
    publication: Arc<Mutex<AeronPublication>>,
    shared: Arc<Shared>,
    request_timeout: Duration,
    running: Arc<AtomicBool>,
    poll_thread: Option<JoinHandle<()>>,
}

impl AeronGatewayClient {
    /// Connect to a gateway on `host` using the default ports and stream ids, using the Aeron media
    /// driver at `aeron_dir`.
    pub fn connect(aeron_dir: &str, host: &str) -> Result<Self, Box<dyn Error>> {
        Self::connect_with(
            aeron_dir,
            &format!("{host}:{DEFAULT_REQUEST_PORT}"),
            DEFAULT_REQUEST_STREAM_ID,
            &format!("{host}:{DEFAULT_RESPONSE_CONTROL_PORT}"),
            DEFAULT_RESPONSE_STREAM_ID,
        )
    }

    /// Connect with fully explicit endpoints and stream ids.
    pub fn connect_with(
        aeron_dir: &str,
        request_endpoint: &str,
        request_stream_id: i32,
        response_control: &str,
        response_stream_id: i32,
    ) -> Result<Self, Box<dyn Error>> {
        let ctx = AeronContext::new()?;
        ctx.set_dir(&cformat!("{aeron_dir}"))?;
        let aeron = Aeron::new(&ctx)?;
        aeron.start()?;

        // 1. Response subscription (control-mode=response).
        let response_channel =
            AeronUriStringBuilder::udp_control(response_control, ControlMode::Response)?
                .build(256)?
                .into_c_string();
        let subscription = aeron
            .async_add_subscription(&response_channel, response_stream_id, Handlers::NONE, Handlers::NONE)?
            .poll_blocking(Duration::from_secs(5))?;

        // 2. Request publication carrying the response subscription's registration id.
        let registration_id = subscription.get_constants()?.registration_id();
        let request_channel = AeronUriStringBuilder::udp(request_endpoint)?
            .response_correlation_id(registration_id)?
            .build(256)?
            .into_c_string();
        let publication = aeron
            .async_add_publication(&request_channel, request_stream_id)?
            .poll_blocking(Duration::from_secs(5))?;

        let shared = Arc::new(Shared::new());
        let running = Arc::new(AtomicBool::new(true));

        // 3. Background poll thread: drains the response subscription (reassembling fragmented
        //    messages such as batched getEntries/getStats) and dispatches decoded frames.
        let poll_thread = {
            let mut shared_ctx = shared.clone();
            let running = running.clone();
            std::thread::spawn(move || {
                let mut assembler = AeronFragmentClosureAssembler::new()
                    .expect("failed to create fragment assembler");
                while running.load(Ordering::Acquire) {
                    let _ = assembler.poll(
                        &subscription,
                        &mut shared_ctx,
                        |shared: &mut Arc<Shared>, buf: &[u8], _hdr| {
                            shared.received_any.store(true, Ordering::Release);
                            dispatch(buf, shared);
                        },
                        FRAGMENT_LIMIT,
                    );
                    std::thread::sleep(Duration::from_millis(1));
                }
            })
        };

        Ok(AeronGatewayClient {
            _ctx: ctx,
            _aeron: aeron,
            publication: Arc::new(Mutex::new(publication)),
            shared,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            running,
            poll_thread: Some(poll_thread),
        })
    }

    /// Block until both the request publication and response subscription are connected, or the timeout
    /// elapses. The gateway creates each client's response publication lazily on the first request it
    /// receives, so this sends harmless `getStats` warmup probes until a response has been observed.
    pub fn await_connected(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            {
                let pub_connected = self.publication.lock().unwrap().is_connected();
                if pub_connected && self.shared.received_any.load(Ordering::Acquire) {
                    return true;
                }
            }
            let frame = encode_command(msg::GET_CACHE_STATS, 0, 0, "connection-warmup", "", "", "");
            let _ = offer_frame(&self.publication, &frame, Duration::from_millis(250));
            std::thread::sleep(Duration::from_millis(250));
        }
        self.publication.lock().unwrap().is_connected() && self.shared.received_any.load(Ordering::Acquire)
    }

    // ------------------------------------------------------------------ cache ops

    pub fn create_cache(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>> {
        let r = self.command(msg::CREATE_CACHE, 0, 0, cache_id, "", "")?;
        Ok(create_response(&r))
    }

    pub fn put_item(&self, cache_id: &str, key: &str, value: &str) -> Result<PutItemResponse, Box<dyn Error>> {
        self.put_timed_item(cache_id, key, value, 0)
    }

    pub fn put_timed_item(&self, cache_id: &str, key: &str, value: &str, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let r = self.command(msg::ADD_CACHE_ENTRY, ttl, 0, cache_id, key, value)?;
        Ok(put_item_response(&r))
    }

    pub fn get_item(&self, cache_id: &str, key: &str) -> Result<GetItemResponse, Box<dyn Error>> {
        let r = self.command(msg::GET_CACHE_ENTRY, 0, 0, cache_id, key, "")?;
        Ok(get_item_response(&r))
    }

    pub fn delete_item(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        let r = self.command(msg::REMOVE_CACHE_ENTRY, 0, 0, cache_id, key, "")?;
        Ok(delete_item_response(&r))
    }

    /// Deep-merge `value` into an existing item (PATCH).
    pub fn patch_item(&self, cache_id: &str, key: &str, value: &str) -> Result<crate::PatchItemResponse, Box<dyn Error>> {
        let r = self.command(msg::PATCH_CACHE_ENTRY, 0, 0, cache_id, key, value)?;
        Ok(crate::PatchItemResponse { cache_id: r.cache_id, key: r.key, operation_status: r.status })
    }

    /// Cancel a scheduled TTL removal of an item.
    pub fn cancel_item_removal(&self, cache_id: &str, key: &str) -> Result<crate::CancelItemRemovalResponse, Box<dyn Error>> {
        let r = self.command(msg::CANCEL_CACHE_ITEM_REMOVAL, 0, 0, cache_id, key, "")?;
        Ok(crate::CancelItemRemovalResponse { cache_id: r.cache_id, key: r.key, operation_status: r.status })
    }

    pub fn delete_cache(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        let r = self.command(msg::DELETE_CACHE, 0, 0, cache_id, "", "")?;
        Ok(delete_cache_response(&r))
    }

    pub fn clear_cache(&self, cache_id: &str) -> Result<crate::ClearCacheResponse, Box<dyn Error>> {
        let r = self.command(msg::CLEAR_CACHE, 0, 0, cache_id, "", "")?;
        Ok(crate::ClearCacheResponse { cache_id: r.cache_id, operation_status: r.status })
    }

    /// Full snapshot of a cache's entries (streamed as one or more batches).
    pub fn get_cache_items(&self, cache_id: &str) -> Result<GetCacheResponse, Box<dyn Error>> {
        let cid = new_correlation_id();
        let (tx, rx) = channel();
        self.shared.pending_entries.lock().unwrap().insert(
            cid.clone(),
            EntriesReq { sender: tx, items: Vec::new(), cache_id: String::new(), status: String::new() },
        );
        let frame = encode_command(msg::GET_CACHE_ENTRIES, 0, 0, &cid, cache_id, "", "");
        offer_frame(&self.publication, &frame, self.request_timeout)?;
        match rx.recv_timeout(self.request_timeout) {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(e)) => { self.shared.pending_entries.lock().unwrap().remove(&cid); Err(e.into()) }
            Err(_) => { self.shared.pending_entries.lock().unwrap().remove(&cid); Err("gateway request timed out".into()) }
        }
    }

    /// Stats for all caches.
    pub fn get_stats(&self) -> Result<Vec<GatewayStat>, Box<dyn Error>> {
        let cid = new_correlation_id();
        let (tx, rx) = channel();
        self.shared.pending_stats.lock().unwrap().insert(cid.clone(), StatsReq { sender: tx, stats: Vec::new() });
        let frame = encode_command(msg::GET_CACHE_STATS, 0, 0, &cid, "", "", "");
        offer_frame(&self.publication, &frame, self.request_timeout)?;
        match rx.recv_timeout(self.request_timeout) {
            Ok(Ok(stats)) => Ok(stats),
            Ok(Err(e)) => { self.shared.pending_stats.lock().unwrap().remove(&cid); Err(e.into()) }
            Err(_) => { self.shared.pending_stats.lock().unwrap().remove(&cid); Err("gateway request timed out".into()) }
        }
    }

    /// Execute a batch of cache operations in a single round-trip, mirroring the HTTP client's
    /// [`crate::AeronCacheClient::bulk_ops`]. The whole batch is correlated by the request's
    /// `request_id` (a fresh id is minted if it is empty); the gateway replies with one reassembled
    /// `GatewayBulkResponse` carrying every per-operation result.
    pub fn bulk_ops(&self, req: &BulkCacheOpsRequest) -> Result<BulkCacheOpsResponse, Box<dyn Error>> {
        let cid = if req.request_id.is_empty() { new_correlation_id() } else { req.request_id.clone() };
        let (tx, rx) = channel();
        self.shared.pending_bulk.lock().unwrap().insert(cid.clone(), tx);
        let frame = encode_bulk_request(&cid, req);
        if let Err(e) = offer_frame(&self.publication, &frame, self.request_timeout) {
            self.shared.pending_bulk.lock().unwrap().remove(&cid);
            return Err(e);
        }
        match rx.recv_timeout(self.request_timeout) {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(msg)) => { self.shared.pending_bulk.lock().unwrap().remove(&cid); Err(msg.into()) }
            Err(_) => { self.shared.pending_bulk.lock().unwrap().remove(&cid); Err("gateway request timed out".into()) }
        }
    }

    // ------------------------------------------------------------------ counter ops

    pub fn create_counter_cache(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>> {
        let r = self.command(msg::CREATE_COUNTER_CACHE, 0, 0, cache_id, "", "")?;
        Ok(create_response(&r))
    }

    pub fn put_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let r = self.command(msg::ADD_COUNTER_ENTRY, 0, value, cache_id, key, "")?;
        Ok(put_item_response(&r))
    }

    pub fn put_timed_counter(&self, cache_id: &str, key: &str, value: i64, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let r = self.command(msg::ADD_COUNTER_ENTRY, ttl, value, cache_id, key, "")?;
        Ok(put_item_response(&r))
    }

    pub fn get_counter(&self, cache_id: &str, key: &str) -> Result<CounterResponse, Box<dyn Error>> {
        let r = self.command(msg::GET_COUNTER_ENTRY, 0, 0, cache_id, key, "")?;
        Ok(counter_response(&r))
    }

    pub fn delete_counter(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        let r = self.command(msg::REMOVE_COUNTER_ENTRY, 0, 0, cache_id, key, "")?;
        Ok(delete_item_response(&r))
    }

    /// Cancel a scheduled TTL removal of a counter.
    pub fn cancel_counter_item_removal(&self, cache_id: &str, key: &str) -> Result<crate::CancelItemRemovalResponse, Box<dyn Error>> {
        let r = self.command(msg::CANCEL_COUNTER_ITEM_REMOVAL, 0, 0, cache_id, key, "")?;
        Ok(crate::CancelItemRemovalResponse { cache_id: r.cache_id, key: r.key, operation_status: r.status })
    }

    pub fn delete_counter_cache(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        let r = self.command(msg::DELETE_COUNTER_CACHE, 0, 0, cache_id, "", "")?;
        Ok(delete_cache_response(&r))
    }

    pub fn increment_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let r = self.command(msg::INCREMENT_COUNTER_ENTRY, 0, amount, cache_id, key, "")?;
        Ok(counter_response(&r))
    }

    pub fn decrement_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let r = self.command(msg::DECREMENT_COUNTER_ENTRY, 0, amount, cache_id, key, "")?;
        Ok(counter_response(&r))
    }

    pub fn set_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let r = self.command(msg::SET_COUNTER_ENTRY, 0, value, cache_id, key, "")?;
        Ok(counter_response(&r))
    }

    // ------------------------------------------------------------------ subscriptions

    /// Subscribe to streaming updates for a cache. `listener` is invoked on the polling thread.
    pub fn subscribe<F>(&self, cache_id: &str, listener: F) -> Result<GatewaySubscription, Box<dyn Error>>
    where
        F: Fn(CacheUpdateEvent) + Send + Sync + 'static,
    {
        self.subscribe_ext(cache_id, false, listener)
    }

    pub fn subscribe_ext<F>(&self, cache_id: &str, hydrate: bool, listener: F) -> Result<GatewaySubscription, Box<dyn Error>>
    where
        F: Fn(CacheUpdateEvent) + Send + Sync + 'static,
    {
        self.subscribe_with(cache_id, hydrate, None, None, listener)
    }

    /// Subscribe to streaming updates for a cache with an optional subscription `mode` and key filter.
    ///
    /// `mode` is `"full"` (default; streams full values as `ADD_ITEM`) or `"patch"` (streams only
    /// changed fields as `PATCH_ITEM`); any other/`None` value maps to `FULL`. `key`, when set,
    /// restricts the subscription to that key. After sending the subscribe frame, this waits
    /// best-effort up to the request timeout for the gateway's `GatewaySubscribeAck`, then proceeds
    /// regardless (older gateways that do not ack still work).
    pub fn subscribe_with<F>(&self, cache_id: &str, hydrate: bool, mode: Option<&str>, key: Option<&str>, listener: F) -> Result<GatewaySubscription, Box<dyn Error>>
    where
        F: Fn(CacheUpdateEvent) + Send + Sync + 'static,
    {
        self.shared.cache_listeners.lock().unwrap()
            .entry(cache_id.to_string()).or_default().push(Arc::new(listener));
        let cid = new_correlation_id();
        self.send_subscribe(&cid, cache_id, hydrate, false, map_mode(mode), key.unwrap_or(""))?;
        Ok(GatewaySubscription {
            cache_id: cache_id.to_string(),
            counters: false,
            publication: self.publication.clone(),
            shared: self.shared.clone(),
        })
    }

    /// Subscribe to streaming updates for a counter cache.
    pub fn subscribe_counter<F>(&self, cache_id: &str, listener: F) -> Result<GatewaySubscription, Box<dyn Error>>
    where
        F: Fn(CounterUpdateEvent) + Send + Sync + 'static,
    {
        self.subscribe_counter_ext(cache_id, false, listener)
    }

    pub fn subscribe_counter_ext<F>(&self, cache_id: &str, hydrate: bool, listener: F) -> Result<GatewaySubscription, Box<dyn Error>>
    where
        F: Fn(CounterUpdateEvent) + Send + Sync + 'static,
    {
        self.shared.counter_listeners.lock().unwrap()
            .entry(cache_id.to_string()).or_default().push(Arc::new(listener));
        // Patch mode is cache-only; counter caches always subscribe in FULL mode with no key filter.
        let cid = new_correlation_id();
        self.send_subscribe(&cid, cache_id, hydrate, true, subscription_mode::SubscriptionMode::FULL, "")?;
        Ok(GatewaySubscription {
            cache_id: cache_id.to_string(),
            counters: true,
            publication: self.publication.clone(),
            shared: self.shared.clone(),
        })
    }

    // ------------------------------------------------------------------ internals

    /// Send a subscribe frame and wait best-effort for its ack (see [`Self::subscribe_with`]).
    fn send_subscribe(&self, cid: &str, cache_id: &str, hydrate: bool, counters: bool,
                      mode: subscription_mode::SubscriptionMode, key: &str) -> Result<(), Box<dyn Error>> {
        let (tx, rx) = channel();
        self.shared.pending_sub_ack.lock().unwrap().insert(cid.to_string(), tx);
        let frame = encode_subscribe(cid, cache_id, hydrate, counters, mode, key);
        if let Err(e) = offer_frame(&self.publication, &frame, self.request_timeout) {
            self.shared.pending_sub_ack.lock().unwrap().remove(cid);
            return Err(e);
        }
        // Best-effort barrier: proceed regardless of whether the ack arrives in time.
        let _ = rx.recv_timeout(self.request_timeout);
        self.shared.pending_sub_ack.lock().unwrap().remove(cid);
        Ok(())
    }

    fn command(&self, msg_type: u16, ttl: i64, counter_value: i64, cache_id: &str, key: &str, value: &str)
        -> Result<CommandResponse, Box<dyn Error>>
    {
        let cid = new_correlation_id();
        let (tx, rx) = channel();
        self.shared.pending_cmd.lock().unwrap().insert(cid.clone(), tx);
        let frame = encode_command(msg_type, ttl, counter_value, &cid, cache_id, key, value);
        if let Err(e) = offer_frame(&self.publication, &frame, self.request_timeout) {
            self.shared.pending_cmd.lock().unwrap().remove(&cid);
            return Err(e);
        }
        match rx.recv_timeout(self.request_timeout) {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(msg)) => Err(msg.into()),
            Err(_) => {
                self.shared.pending_cmd.lock().unwrap().remove(&cid);
                Err("gateway request timed out".into())
            }
        }
    }
}

impl Drop for AeronGatewayClient {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(handle) = self.poll_thread.take() {
            let _ = handle.join();
        }
    }
}

// ------------------------------------------------------------------ encoding

fn new_correlation_id() -> String {
    // A lightweight unique id (no uuid dependency): time + counter.
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}-{n:x}")
}

fn encode_command(msg_type: u16, ttl: i64, counter_value: i64, correlation_id: &str, cache_id: &str, key: &str, value: &str) -> Vec<u8> {
    let cap = message_header_codec::ENCODED_LENGTH + 32 + correlation_id.len() + cache_id.len() + key.len() + value.len() + 16;
    let mut buf = vec![0u8; cap];
    let len;
    {
        let mut enc = gateway_command_codec::encoder::GatewayCommandEncoder::default()
            .wrap(WriteBuf::new(&mut buf), message_header_codec::ENCODED_LENGTH);
        enc = enc.header(0).parent().unwrap();
        enc.msg_type(msg_type);
        enc.ttl(ttl);
        enc.counter_value(counter_value);
        enc.correlation_id(correlation_id);
        enc.cache_id(cache_id);
        enc.key(key);
        enc.value(value);
        len = message_header_codec::ENCODED_LENGTH + enc.encoded_length();
    }
    buf.truncate(len);
    buf
}

fn encode_subscribe(correlation_id: &str, cache_id: &str, send_snapshot: bool, counters: bool,
                    mode: subscription_mode::SubscriptionMode, key: &str) -> Vec<u8> {
    let cap = message_header_codec::ENCODED_LENGTH + 64 + correlation_id.len() + cache_id.len() + key.len();
    let mut buf = vec![0u8; cap];
    let len;
    {
        use gateway_subscribe_codec::encoder::{CacheIdsEncoder, GatewaySubscribeEncoder};
        let mut enc = GatewaySubscribeEncoder::default()
            .wrap(WriteBuf::new(&mut buf), message_header_codec::ENCODED_LENGTH);
        enc = enc.header(0).parent().unwrap();
        enc.send_snapshot(map_bool(send_snapshot));
        enc.counters(map_bool(counters));
        let mut group = enc.cache_ids_encoder(1, CacheIdsEncoder::default());
        group.advance().unwrap();
        group.mode(mode);
        group.cache_id(cache_id);
        group.key(key);
        let mut enc = group.parent().unwrap();
        enc.correlation_id(correlation_id);
        len = message_header_codec::ENCODED_LENGTH + enc.encoded_length();
    }
    buf.truncate(len);
    buf
}

/// Encode a `GatewayBulkRequest` frame from an HTTP-model bulk request.
fn encode_bulk_request(correlation_id: &str, req: &BulkCacheOpsRequest) -> Vec<u8> {
    let mut cap = message_header_codec::ENCODED_LENGTH + 8 + correlation_id.len() + 16;
    for op in &req.operations {
        cap += 17 + 16
            + op.request_id.len() + op.cache_id.len()
            + op.key.as_deref().unwrap_or("").len()
            + op.value.as_deref().unwrap_or("").len();
    }
    let mut buf = vec![0u8; cap];
    let len;
    {
        use gateway_bulk_request_codec::encoder::{GatewayBulkRequestEncoder, OperationsEncoder};
        let mut enc = GatewayBulkRequestEncoder::default()
            .wrap(WriteBuf::new(&mut buf), message_header_codec::ENCODED_LENGTH);
        enc = enc.header(0).parent().unwrap();
        let mut group = enc.operations_encoder(req.operations.len() as u16, OperationsEncoder::default());
        for op in &req.operations {
            group.advance().unwrap();
            group.operation_type(map_bulk_op(&op.operation_type));
            group.ttl(op.ttl.unwrap_or(0));
            group.counter_value(op.counter_value.unwrap_or(0));
            group.request_id(&op.request_id);
            group.cache_id(&op.cache_id);
            group.key(op.key.as_deref().unwrap_or(""));
            group.value(op.value.as_deref().unwrap_or(""));
        }
        let mut enc = group.parent().unwrap();
        enc.correlation_id(correlation_id);
        len = message_header_codec::ENCODED_LENGTH + enc.encoded_length();
    }
    buf.truncate(len);
    buf
}

/// Map the HTTP `BulkOperationType` model onto the SBE `bulk_operation_type` enum (by name; the
/// SBE enum has extra variants such as `PATCH_ITEM` that shift the ordinals).
fn map_bulk_op(t: &crate::BulkOperationType) -> bulk_operation_type::BulkOperationType {
    use bulk_operation_type::BulkOperationType as S;
    use crate::BulkOperationType as H;
    match t {
        H::None => S::NONE,
        H::CreateCache => S::CREATE_CACHE,
        H::AddItem => S::ADD_ITEM,
        H::RemoveItem => S::REMOVE_ITEM,
        H::ClearCache => S::CLEAR_CACHE,
        H::GetItem => S::GET_ITEM,
        H::DeleteCache => S::DELETE_CACHE,
        H::CreateCounterCache => S::CREATE_COUNTER_CACHE,
        H::AddCounter => S::ADD_COUNTER,
        H::RemoveCounter => S::REMOVE_COUNTER,
        H::ClearCounterCache => S::CLEAR_COUNTER_CACHE,
        H::GetCounter => S::GET_COUNTER,
        H::DeleteCounterCache => S::DELETE_COUNTER_CACHE,
        H::IncrementCounter => S::INCREMENT_COUNTER,
        H::DecrementCounter => S::DECREMENT_COUNTER,
        H::SetCounter => S::SET_COUNTER,
    }
}

/// Map an optional subscription mode string onto the SBE enum (`"patch"` → PATCH, else FULL).
fn map_mode(mode: Option<&str>) -> subscription_mode::SubscriptionMode {
    match mode {
        Some(m) if m.eq_ignore_ascii_case("patch") => subscription_mode::SubscriptionMode::PATCH,
        _ => subscription_mode::SubscriptionMode::FULL,
    }
}

fn encode_unsubscribe(correlation_id: &str, cache_id: &str, counters: bool) -> Vec<u8> {
    let cap = message_header_codec::ENCODED_LENGTH + 32 + correlation_id.len() + cache_id.len();
    let mut buf = vec![0u8; cap];
    let len;
    {
        let mut enc = gateway_unsubscribe_codec::encoder::GatewayUnsubscribeEncoder::default()
            .wrap(WriteBuf::new(&mut buf), message_header_codec::ENCODED_LENGTH);
        enc = enc.header(0).parent().unwrap();
        enc.counters(map_bool(counters));
        enc.correlation_id(correlation_id);
        enc.cache_id(cache_id);
        len = message_header_codec::ENCODED_LENGTH + enc.encoded_length();
    }
    buf.truncate(len);
    buf
}

fn map_bool(v: bool) -> boolean_type::BooleanType {
    if v { boolean_type::BooleanType::T } else { boolean_type::BooleanType::F }
}

fn offer_frame(publication: &Arc<Mutex<AeronPublication>>, frame: &[u8], timeout: Duration) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + timeout;
    let pubn = publication.lock().unwrap();
    loop {
        match pubn.offer(frame) {
            Ok(_) => return Ok(()),
            Err(e) if e.is_retryable() && Instant::now() < deadline => std::thread::sleep(Duration::from_millis(1)),
            Err(e) => return Err(format!("gateway offer failed: {e}").into()),
        }
    }
}

// ------------------------------------------------------------------ decoding / dispatch

fn dispatch(data: &[u8], shared: &Shared) {
    let header = message_header_codec::decoder::MessageHeaderDecoder::default()
        .wrap(ReadBuf::new(data), 0);
    let template_id = header.template_id();

    match template_id {
        gateway_command_response_codec::SBE_TEMPLATE_ID => {
            let mut dec = gateway_command_response_codec::decoder::GatewayCommandResponseDecoder::default().header(header);
            let status = status_name(dec.status());
            let correlation_id = read_str(dec.correlation_id_decoder(), |c| dec.correlation_id_slice(c));
            let cache_id = read_str(dec.cache_id_decoder(), |c| dec.cache_id_slice(c));
            let key = read_str(dec.key_decoder(), |c| dec.key_slice(c));
            let value = read_str(dec.value_decoder(), |c| dec.value_slice(c));
            if let Some(tx) = shared.pending_cmd.lock().unwrap().remove(&correlation_id) {
                let _ = tx.send(Ok(CommandResponse { status, cache_id, key, value }));
            }
        }
        gateway_error_codec::SBE_TEMPLATE_ID => {
            let mut dec = gateway_error_codec::decoder::GatewayErrorDecoder::default().header(header);
            let status = status_name(dec.status());
            let correlation_id = read_str(dec.correlation_id_decoder(), |c| dec.correlation_id_slice(c));
            let message = read_str(dec.message_decoder(), |c| dec.message_slice(c));
            let err = format!("gateway error [{status}]: {message}");
            if let Some(tx) = shared.pending_cmd.lock().unwrap().remove(&correlation_id) {
                let _ = tx.send(Err(err.clone()));
            } else if let Some(req) = shared.pending_entries.lock().unwrap().remove(&correlation_id) {
                let _ = req.sender.send(Err(err.clone()));
            } else if let Some(req) = shared.pending_stats.lock().unwrap().remove(&correlation_id) {
                let _ = req.sender.send(Err(err.clone()));
            } else if let Some(tx) = shared.pending_bulk.lock().unwrap().remove(&correlation_id) {
                let _ = tx.send(Err(err));
            }
        }
        gateway_stream_update_codec::SBE_TEMPLATE_ID => {
            let mut dec = gateway_stream_update_codec::decoder::GatewayStreamUpdateDecoder::default().header(header);
            let event_type = event_type_name(dec.event_type());
            let cache_id = read_str(dec.cache_id_decoder(), |c| dec.cache_id_slice(c));
            let key = read_str(dec.key_decoder(), |c| dec.key_slice(c));
            let value = read_str(dec.value_decoder(), |c| dec.value_slice(c));
            let correlation_id = read_str(dec.correlation_id_decoder(), |c| dec.correlation_id_slice(c));
            dispatch_stream_update(shared, &event_type, &cache_id, &key, &value, &correlation_id);
        }
        gateway_entries_codec::SBE_TEMPLATE_ID => {
            decode_entries(header, shared);
        }
        gateway_stats_codec::SBE_TEMPLATE_ID => {
            decode_stats(header, shared);
        }
        gateway_subscribe_ack_codec::SBE_TEMPLATE_ID => {
            decode_subscribe_ack(header, shared);
        }
        gateway_bulk_response_codec::SBE_TEMPLATE_ID => {
            decode_bulk_response(header, shared);
        }
        _ => {}
    }
}

fn decode_subscribe_ack(header: message_header_codec::decoder::MessageHeaderDecoder<ReadBuf>, shared: &Shared) {
    let dec = gateway_subscribe_ack_codec::decoder::GatewaySubscribeAckDecoder::default().header(header);
    let _status = status_name(dec.status());
    // Consume the cacheIds group to reach the trailing correlation id.
    let mut group = dec.cache_ids_decoder();
    while let Ok(Some(_)) = group.advance() {
        let _ = read_str(group.cache_id_decoder(), |c| group.cache_id_slice(c));
    }
    let mut dec = group.parent().unwrap();
    let correlation_id = read_str(dec.correlation_id_decoder(), |c| dec.correlation_id_slice(c));
    if let Some(tx) = shared.pending_sub_ack.lock().unwrap().remove(&correlation_id) {
        let _ = tx.send(());
    }
}

fn decode_bulk_response(header: message_header_codec::decoder::MessageHeaderDecoder<ReadBuf>, shared: &Shared) {
    let dec = gateway_bulk_response_codec::decoder::GatewayBulkResponseDecoder::default().header(header);

    let mut responses: Vec<CacheOperationResponse> = Vec::new();
    let mut ops = dec.operations_decoder();
    while let Ok(Some(_)) = ops.advance() {
        let status = status_name(ops.status());
        let request_id = read_str(ops.request_id_decoder(), |c| ops.request_id_slice(c));
        let cache_id = read_str(ops.cache_id_decoder(), |c| ops.cache_id_slice(c));
        let key = read_str(ops.key_decoder(), |c| ops.key_slice(c));
        let value = read_str(ops.value_decoder(), |c| ops.value_slice(c));
        responses.push(CacheOperationResponse {
            request_id,
            status,
            cache_id,
            key: if key.is_empty() { None } else { Some(key) },
            value: if value.is_empty() { None } else { Some(value) },
        });
    }
    let mut dec = ops.parent().unwrap();
    let correlation_id = read_str(dec.correlation_id_decoder(), |c| dec.correlation_id_slice(c));

    if let Some(tx) = shared.pending_bulk.lock().unwrap().remove(&correlation_id) {
        let _ = tx.send(Ok(BulkCacheOpsResponse {
            request_id: correlation_id,
            operation_responses: responses,
        }));
    }
}

fn dispatch_stream_update(shared: &Shared, event_type: &str, cache_id: &str, key: &str, value: &str, correlation_id: &str) {
    let opt_key = if key.is_empty() { None } else { Some(key.to_string()) };
    {
        let listeners = shared.cache_listeners.lock().unwrap();
        if let Some(list) = listeners.get(cache_id) {
            let event = CacheUpdateEvent {
                cache_id: cache_id.to_string(),
                event_type: event_type.to_string(),
                item_key: opt_key.clone(),
                item_value: if value.is_empty() { None } else { Some(value.to_string()) },
                request_id: correlation_id.to_string(),
            };
            for l in list.iter() { l(event.clone()); }
        }
    }
    {
        let listeners = shared.counter_listeners.lock().unwrap();
        if let Some(list) = listeners.get(cache_id) {
            let event = CounterUpdateEvent {
                cache_id: cache_id.to_string(),
                event_type: event_type.to_string(),
                item_key: opt_key,
                item_value: value.parse::<i64>().ok(),
                request_id: correlation_id.to_string(),
            };
            for l in list.iter() { l(event.clone()); }
        }
    }
}

fn decode_entries(header: message_header_codec::decoder::MessageHeaderDecoder<ReadBuf>, shared: &Shared) {
    let dec = gateway_entries_codec::decoder::GatewayEntriesDecoder::default().header(header);
    let status = status_name(dec.status());
    let end_of_batch = matches!(dec.end_of_batch(), boolean_type::BooleanType::T);

    let mut batch: Vec<CacheItem> = Vec::new();
    let mut items = dec.items_decoder();
    while let Ok(Some(_)) = items.advance() {
        let key = read_str(items.key_decoder(), |c| items.key_slice(c));
        let value = read_str(items.value_decoder(), |c| items.value_slice(c));
        batch.push(CacheItem { key, value });
    }
    let mut dec = items.parent().unwrap();
    let correlation_id = read_str(dec.correlation_id_decoder(), |c| dec.correlation_id_slice(c));
    let cache_id = read_str(dec.cache_id_decoder(), |c| dec.cache_id_slice(c));

    let mut map = shared.pending_entries.lock().unwrap();
    if let Some(req) = map.get_mut(&correlation_id) {
        req.items.extend(batch);
        req.cache_id = cache_id;
        req.status = status;
        if end_of_batch {
            let req = map.remove(&correlation_id).unwrap();
            let _ = req.sender.send(Ok(GetCacheResponse {
                cache_id: req.cache_id,
                operation_status: req.status,
                items: req.items,
            }));
        }
    }
}

fn decode_stats(header: message_header_codec::decoder::MessageHeaderDecoder<ReadBuf>, shared: &Shared) {
    let dec = gateway_stats_codec::decoder::GatewayStatsDecoder::default().header(header);
    let _status = status_name(dec.status());
    let end_of_batch = matches!(dec.end_of_batch(), boolean_type::BooleanType::T);

    let mut batch: Vec<GatewayStat> = Vec::new();
    let mut stats = dec.stats_decoder();
    while let Ok(Some(_)) = stats.advance() {
        let added = stats.added_count();
        let removed = stats.removed_count();
        let cleared = stats.cleared_count();
        let size = stats.size();
        let cache_id = read_str(stats.cache_id_decoder(), |c| stats.cache_id_slice(c));
        batch.push(GatewayStat { cache_id, added_count: added, removed_count: removed, cleared_count: cleared, size });
    }
    let mut dec = stats.parent().unwrap();
    let correlation_id = read_str(dec.correlation_id_decoder(), |c| dec.correlation_id_slice(c));

    let mut map = shared.pending_stats.lock().unwrap();
    if let Some(req) = map.get_mut(&correlation_id) {
        req.stats.extend(batch);
        if end_of_batch {
            let req = map.remove(&correlation_id).unwrap();
            let _ = req.sender.send(Ok(req.stats));
        }
    }
}

/// Read a var-string given its `(offset, length)` coordinates and a slice accessor.
fn read_str<'a, F>(coords: (usize, usize), slice: F) -> String
where
    F: FnOnce((usize, usize)) -> &'a [u8],
{
    String::from_utf8_lossy(slice(coords)).into_owned()
}

fn status_name(s: operation_status::OperationStatus) -> String {
    format!("{s:?}")
}

fn event_type_name(e: update_event_type::UpdateEventType) -> String {
    format!("{e:?}")
}

// ------------------------------------------------------------------ response model mapping

fn create_response(r: &CommandResponse) -> CreateResponse {
    CreateResponse { cache_id: r.cache_id.clone(), operation_status: r.status.clone() }
}

fn put_item_response(r: &CommandResponse) -> PutItemResponse {
    PutItemResponse { cache_id: r.cache_id.clone(), key: r.key.clone(), status: r.status.clone(), operation_status: r.status.clone() }
}

fn get_item_response(r: &CommandResponse) -> GetItemResponse {
    GetItemResponse { cache_id: r.cache_id.clone(), key: r.key.clone(), value: r.value.clone(), operation_status: r.status.clone() }
}

fn delete_item_response(r: &CommandResponse) -> DeleteItemResponse {
    DeleteItemResponse { cache_id: r.cache_id.clone(), key: r.key.clone(), operation_status: r.status.clone() }
}

fn delete_cache_response(r: &CommandResponse) -> DeleteCacheResponse {
    DeleteCacheResponse { cache_id: r.cache_id.clone(), operation_status: r.status.clone() }
}

fn counter_response(r: &CommandResponse) -> CounterResponse {
    CounterResponse {
        cache_id: r.cache_id.clone(),
        key: r.key.clone(),
        value: r.value.parse::<i64>().unwrap_or(0),
        operation_status: r.status.clone(),
    }
}

// ------------------------------------------------------------------ unit tests (SBE round-trip)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BulkCacheOpsRequest, CacheOperationRequest};

    fn decode_header(frame: &[u8]) -> message_header_codec::decoder::MessageHeaderDecoder<ReadBuf<'_>> {
        message_header_codec::decoder::MessageHeaderDecoder::default().wrap(ReadBuf::new(frame), 0)
    }

    #[test]
    fn bulk_request_round_trips_operations_and_correlation_id() {
        let req = BulkCacheOpsRequest {
            request_id: "batch-1".to_string(),
            operations: vec![
                CacheOperationRequest {
                    operation_type: crate::BulkOperationType::AddItem,
                    request_id: "r1".to_string(),
                    cache_id: "c1".to_string(),
                    key: Some("k1".to_string()),
                    value: Some("v1".to_string()),
                    ttl: Some(1234),
                    counter_value: None,
                },
                CacheOperationRequest {
                    operation_type: crate::BulkOperationType::IncrementCounter,
                    request_id: "r2".to_string(),
                    cache_id: "c2".to_string(),
                    key: Some("k2".to_string()),
                    value: None,
                    ttl: None,
                    counter_value: Some(7),
                },
            ],
        };
        let frame = encode_bulk_request("batch-1", &req);

        let header = decode_header(&frame);
        assert_eq!(header.template_id(), gateway_bulk_request_codec::SBE_TEMPLATE_ID);
        let dec = gateway_bulk_request_codec::decoder::GatewayBulkRequestDecoder::default().header(header);
        let mut ops = dec.operations_decoder();
        assert_eq!(ops.count(), 2);

        assert!(ops.advance().unwrap().is_some());
        // IncrementCounter is ordinal 13 in the HTTP model but 14 in SBE (PATCH_ITEM shifts it):
        // the by-name mapping must survive that.
        assert_eq!(ops.operation_type(), bulk_operation_type::BulkOperationType::ADD_ITEM);
        assert_eq!(ops.ttl(), 1234);
        assert_eq!(ops.counter_value(), 0);
        assert_eq!(read_str(ops.request_id_decoder(), |c| ops.request_id_slice(c)), "r1");
        assert_eq!(read_str(ops.cache_id_decoder(), |c| ops.cache_id_slice(c)), "c1");
        assert_eq!(read_str(ops.key_decoder(), |c| ops.key_slice(c)), "k1");
        assert_eq!(read_str(ops.value_decoder(), |c| ops.value_slice(c)), "v1");

        assert!(ops.advance().unwrap().is_some());
        assert_eq!(ops.operation_type(), bulk_operation_type::BulkOperationType::INCREMENT_COUNTER);
        assert_eq!(ops.counter_value(), 7);
        assert_eq!(read_str(ops.request_id_decoder(), |c| ops.request_id_slice(c)), "r2");
        assert_eq!(read_str(ops.cache_id_decoder(), |c| ops.cache_id_slice(c)), "c2");
        assert_eq!(read_str(ops.key_decoder(), |c| ops.key_slice(c)), "k2");
        assert_eq!(read_str(ops.value_decoder(), |c| ops.value_slice(c)), "");

        assert!(ops.advance().unwrap().is_none());
        let mut dec = ops.parent().unwrap();
        assert_eq!(read_str(dec.correlation_id_decoder(), |c| dec.correlation_id_slice(c)), "batch-1");
    }

    #[test]
    fn subscribe_encodes_patch_mode_and_key() {
        let frame = encode_subscribe(
            "sub-1", "cacheA", true, false,
            subscription_mode::SubscriptionMode::PATCH, "mykey",
        );
        let header = decode_header(&frame);
        assert_eq!(header.template_id(), gateway_subscribe_codec::SBE_TEMPLATE_ID);
        let dec = gateway_subscribe_codec::decoder::GatewaySubscribeDecoder::default().header(header);
        assert!(matches!(dec.send_snapshot(), boolean_type::BooleanType::T));
        assert!(matches!(dec.counters(), boolean_type::BooleanType::F));
        let mut group = dec.cache_ids_decoder();
        assert!(group.advance().unwrap().is_some());
        assert_eq!(group.mode(), subscription_mode::SubscriptionMode::PATCH);
        assert_eq!(read_str(group.cache_id_decoder(), |c| group.cache_id_slice(c)), "cacheA");
        assert_eq!(read_str(group.key_decoder(), |c| group.key_slice(c)), "mykey");
        assert!(group.advance().unwrap().is_none());
        let mut dec = group.parent().unwrap();
        assert_eq!(read_str(dec.correlation_id_decoder(), |c| dec.correlation_id_slice(c)), "sub-1");
    }

    #[test]
    fn subscribe_defaults_to_full_mode_and_empty_key() {
        let frame = encode_subscribe(
            "sub-2", "cacheB", false, true,
            subscription_mode::SubscriptionMode::FULL, "",
        );
        let header = decode_header(&frame);
        let dec = gateway_subscribe_codec::decoder::GatewaySubscribeDecoder::default().header(header);
        let mut group = dec.cache_ids_decoder();
        assert!(group.advance().unwrap().is_some());
        assert_eq!(group.mode(), subscription_mode::SubscriptionMode::FULL);
        assert_eq!(read_str(group.cache_id_decoder(), |c| group.cache_id_slice(c)), "cacheB");
        assert_eq!(read_str(group.key_decoder(), |c| group.key_slice(c)), "");
    }

    #[test]
    fn map_mode_recognizes_patch_case_insensitively() {
        assert_eq!(map_mode(Some("patch")), subscription_mode::SubscriptionMode::PATCH);
        assert_eq!(map_mode(Some("PATCH")), subscription_mode::SubscriptionMode::PATCH);
        assert_eq!(map_mode(Some("full")), subscription_mode::SubscriptionMode::FULL);
        assert_eq!(map_mode(None), subscription_mode::SubscriptionMode::FULL);
    }

    /// Encode a `GatewayBulkResponse` the way the gateway would, for decode/dispatch tests.
    fn encode_bulk_response(
        correlation_id: &str,
        ops: &[(operation_status::OperationStatus, &str, &str, &str, &str)],
    ) -> Vec<u8> {
        let mut cap = message_header_codec::ENCODED_LENGTH + 8 + correlation_id.len() + 16;
        for (_, rid, cid, k, v) in ops {
            cap += 1 + 16 + rid.len() + cid.len() + k.len() + v.len();
        }
        let mut buf = vec![0u8; cap];
        let len;
        {
            use gateway_bulk_response_codec::encoder::{GatewayBulkResponseEncoder, OperationsEncoder};
            let mut enc = GatewayBulkResponseEncoder::default()
                .wrap(WriteBuf::new(&mut buf), message_header_codec::ENCODED_LENGTH);
            enc = enc.header(0).parent().unwrap();
            let mut group = enc.operations_encoder(ops.len() as u16, OperationsEncoder::default());
            for (status, rid, cid, k, v) in ops {
                group.advance().unwrap();
                group.status(*status);
                group.request_id(rid);
                group.cache_id(cid);
                group.key(k);
                group.value(v);
            }
            let mut enc = group.parent().unwrap();
            enc.correlation_id(correlation_id);
            len = message_header_codec::ENCODED_LENGTH + enc.encoded_length();
        }
        buf.truncate(len);
        buf
    }

    #[test]
    fn bulk_response_decodes_and_routes_to_pending() {
        let shared = Shared::new();
        let (tx, rx) = channel();
        shared.pending_bulk.lock().unwrap().insert("batch-9".to_string(), tx);

        let frame = encode_bulk_response("batch-9", &[
            (operation_status::OperationStatus::SUCCESS, "r1", "c1", "k1", "v1"),
            (operation_status::OperationStatus::UNKNOWN_KEY, "r2", "c2", "", ""),
        ]);
        dispatch(&frame, &shared);

        let resp = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        assert_eq!(resp.request_id, "batch-9");
        assert_eq!(resp.operation_responses.len(), 2);
        assert_eq!(resp.operation_responses[0].status, "SUCCESS");
        assert_eq!(resp.operation_responses[0].cache_id, "c1");
        assert_eq!(resp.operation_responses[0].key.as_deref(), Some("k1"));
        assert_eq!(resp.operation_responses[0].value.as_deref(), Some("v1"));
        assert_eq!(resp.operation_responses[1].request_id, "r2");
        assert_eq!(resp.operation_responses[1].status, "UNKNOWN_KEY");
        assert_eq!(resp.operation_responses[1].key, None);
        assert_eq!(resp.operation_responses[1].value, None);
    }

    /// Encode a `GatewaySubscribeAck` the way the gateway would, for decode/dispatch tests.
    fn encode_subscribe_ack(
        correlation_id: &str,
        status: operation_status::OperationStatus,
        cache_ids: &[&str],
    ) -> Vec<u8> {
        let mut cap = message_header_codec::ENCODED_LENGTH + 8 + correlation_id.len() + 16;
        for c in cache_ids {
            cap += 4 + c.len();
        }
        let mut buf = vec![0u8; cap];
        let len;
        {
            use gateway_subscribe_ack_codec::encoder::{CacheIdsEncoder, GatewaySubscribeAckEncoder};
            let mut enc = GatewaySubscribeAckEncoder::default()
                .wrap(WriteBuf::new(&mut buf), message_header_codec::ENCODED_LENGTH);
            enc = enc.header(0).parent().unwrap();
            enc.status(status);
            let mut group = enc.cache_ids_encoder(cache_ids.len() as u16, CacheIdsEncoder::default());
            for c in cache_ids {
                group.advance().unwrap();
                group.cache_id(c);
            }
            let mut enc = group.parent().unwrap();
            enc.correlation_id(correlation_id);
            len = message_header_codec::ENCODED_LENGTH + enc.encoded_length();
        }
        buf.truncate(len);
        buf
    }

    #[test]
    fn subscribe_ack_decodes_and_completes_barrier() {
        let shared = Shared::new();
        let (tx, rx) = channel();
        shared.pending_sub_ack.lock().unwrap().insert("sub-7".to_string(), tx);

        let frame = encode_subscribe_ack(
            "sub-7",
            operation_status::OperationStatus::SUCCESS,
            &["cacheA", "cacheB"],
        );
        dispatch(&frame, &shared);

        assert!(rx.recv_timeout(Duration::from_secs(1)).is_ok());
    }
}
