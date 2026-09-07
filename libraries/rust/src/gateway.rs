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
    boolean_type, gateway_command_codec, gateway_command_response_codec, gateway_entries_codec,
    gateway_error_codec, gateway_stats_codec, gateway_stream_update_codec, gateway_subscribe_codec,
    gateway_unsubscribe_codec, operation_status, update_event_type, ReadBuf, WriteBuf,
};
use crate::{
    CacheItem, CacheUpdateEvent, CounterResponse, CounterUpdateEvent, CreateResponse,
    DeleteCacheResponse, DeleteItemResponse, GetCacheResponse, GetItemResponse, PutItemResponse,
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

    pub const CREATE_COUNTER_CACHE: u16 = 101;
    pub const ADD_COUNTER_ENTRY: u16 = 102;
    pub const GET_COUNTER_ENTRY: u16 = 103;
    pub const DELETE_COUNTER_CACHE: u16 = 105;
    pub const REMOVE_COUNTER_ENTRY: u16 = 110;
    pub const INCREMENT_COUNTER_ENTRY: u16 = 111;
    pub const DECREMENT_COUNTER_ENTRY: u16 = 112;
    pub const SET_COUNTER_ENTRY: u16 = 113;
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
        self.shared.cache_listeners.lock().unwrap()
            .entry(cache_id.to_string()).or_default().push(Arc::new(listener));
        let frame = encode_subscribe(&new_correlation_id(), cache_id, hydrate, false);
        offer_frame(&self.publication, &frame, self.request_timeout)?;
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
        let frame = encode_subscribe(&new_correlation_id(), cache_id, hydrate, true);
        offer_frame(&self.publication, &frame, self.request_timeout)?;
        Ok(GatewaySubscription {
            cache_id: cache_id.to_string(),
            counters: true,
            publication: self.publication.clone(),
            shared: self.shared.clone(),
        })
    }

    // ------------------------------------------------------------------ internals

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

fn encode_subscribe(correlation_id: &str, cache_id: &str, send_snapshot: bool, counters: bool) -> Vec<u8> {
    let cap = message_header_codec::ENCODED_LENGTH + 64 + correlation_id.len() + cache_id.len();
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
        group.cache_id(cache_id);
        let mut enc = group.parent().unwrap();
        enc.correlation_id(correlation_id);
        len = message_header_codec::ENCODED_LENGTH + enc.encoded_length();
    }
    buf.truncate(len);
    buf
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
                let _ = req.sender.send(Err(err));
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
        _ => {}
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
