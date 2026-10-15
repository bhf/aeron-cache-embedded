//! Long-running client soak — a faithful port of the Java client soak (`CacheSoakTest`).
//!
//! Drives a weighted, seeded-random mix of key-value and counter operations through one client
//! transport (`http`, `bidi` or `gateway`) against a running backend, reconciling against an
//! in-memory oracle and sampling process memory throughout. Correctness drift, a stalled op, or
//! resident memory that trends up with time are soak findings. The run is purely time-bounded; the
//! resolved seed is recorded in the report so any finding is reproducible.
//!
//! Gated on `SOAK_ENABLED=true` (set by the `Soak Rust` workflow) so it never runs under a normal
//! `cargo test`. Transport is selected by `SOAK_TRANSPORT` (`http`|`bidi`|`gateway`). HTTP/bidi
//! additionally require `AERON_CACHE_BASE_URL`/`AERON_CACHE_WS_URL` and skip (green) when they are
//! unset; the gateway transport launches its own embedded media driver and connects over UDP to the
//! gateway at `AERON_GATEWAY_HOST` (default `127.0.0.1`).
//!
//! It lives in `tests/` (rather than a `[[bin]]`) so it can use the `rusteron-media-driver`
//! dev-dependency for the gateway transport's embedded media driver, exactly like the gateway
//! integration tests.

use aeron_cache_embedded_client::{AeronBidiClient, AeronCacheClient, AeronGatewayClient};
use rusteron_media_driver::testing::EmbeddedDriver;
use serde::Serialize;
use std::collections::BTreeMap;
use std::error::Error;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type BoxErr = Box<dyn Error>;

// ----------------------------------------------------------------------- deterministic PRNG

/// A small deterministic PRNG (SplitMix64) so a soak run is reproducible from its recorded seed; the
/// crate deliberately pulls in no `rand` dependency. Good enough for weighted op selection and
/// key/value fuzzing (a workload driver, not a cryptographic context).
struct SoakRandom {
    state: u64,
}

impl SoakRandom {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Next value in `[0, bound)`.
    fn next_int(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }

    /// Next `i64` in `[0, bound)`.
    fn next_i64(&mut self, bound: i64) -> i64 {
        (self.next_u64() % bound as u64) as i64
    }
}

// ----------------------------------------------------------------------- config

/// Configuration for a soak run, resolved from `SOAK_*` environment variables with sensible defaults
/// mirroring the Java soak. The resolved values — crucially the seed — are recorded in the report so
/// any failure is reproducible by re-running with the same `SOAK_SEED`.
#[derive(Clone)]
struct SoakConfig {
    transport: String,
    duration_seconds: u64,
    seed: u64,
    kv_cache_count: usize,
    counter_cache_count: usize,
    key_space: usize,
    value_size_bytes: usize,
    verify_every: u64,
    op_timeout_seconds: u64,
}

impl SoakConfig {
    fn from_env() -> Self {
        Self {
            transport: str_env("SOAK_TRANSPORT", "http"),
            duration_seconds: int_env("SOAK_DURATION_SECONDS", 1800),
            seed: seed_env("SOAK_SEED"),
            kv_cache_count: int_env("SOAK_KV_CACHE_COUNT", 4) as usize,
            counter_cache_count: int_env("SOAK_COUNTER_CACHE_COUNT", 4) as usize,
            key_space: int_env("SOAK_KEY_SPACE", 500) as usize,
            value_size_bytes: int_env("SOAK_VALUE_SIZE_BYTES", 160) as usize,
            verify_every: int_env("SOAK_VERIFY_EVERY", 2000),
            op_timeout_seconds: int_env("SOAK_OP_TIMEOUT_SECONDS", 30),
        }
    }

    fn to_report_config(&self) -> ReportConfig {
        ReportConfig {
            transport: self.transport.clone(),
            kv_cache_count: self.kv_cache_count,
            counter_cache_count: self.counter_cache_count,
            key_space: self.key_space,
            value_size_bytes: self.value_size_bytes,
            verify_every: self.verify_every,
            op_timeout_seconds: self.op_timeout_seconds,
        }
    }
}

fn str_env(key: &str, default: &str) -> String {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => default.to_string(),
    }
}

fn int_env(key: &str, default: u64) -> u64 {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => v.trim().parse().expect("invalid integer env var"),
        _ => default,
    }
}

/// A blank or missing seed yields a fresh seed derived from the clock (recorded for reproduction).
fn seed_env(key: &str) -> u64 {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => v.trim().parse().expect("invalid seed env var"),
        _ => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
    }
}

// ----------------------------------------------------------------------- oracle

/// The in-memory model kept in lock-step with the backend. Bounded by the configured key space, so it
/// does not itself grow with time; any divergence from what the client reads back is a soak finding.
#[derive(Default)]
struct SoakOracle {
    kv: BTreeMap<String, BTreeMap<String, String>>,
    counters: BTreeMap<String, BTreeMap<String, i64>>,
}

impl SoakOracle {
    fn register_kv_cache(&mut self, cache_id: &str) {
        self.kv.insert(cache_id.to_string(), BTreeMap::new());
    }
    fn register_counter_cache(&mut self, cache_id: &str) {
        self.counters.insert(cache_id.to_string(), BTreeMap::new());
    }
    fn put_kv(&mut self, cache_id: &str, key: &str, value: &str) {
        self.kv.get_mut(cache_id).unwrap().insert(key.to_string(), value.to_string());
    }
    fn remove_kv(&mut self, cache_id: &str, key: &str) {
        self.kv.get_mut(cache_id).unwrap().remove(key);
    }
    fn clear_kv(&mut self, cache_id: &str) {
        self.kv.get_mut(cache_id).unwrap().clear();
    }
    fn get_kv(&self, cache_id: &str, key: &str) -> Option<String> {
        self.kv.get(cache_id).unwrap().get(key).cloned()
    }
    fn kv_keys(&self, cache_id: &str) -> Vec<String> {
        self.kv.get(cache_id).unwrap().keys().cloned().collect()
    }
    fn kv_snapshot(&self, cache_id: &str) -> BTreeMap<String, String> {
        self.kv.get(cache_id).unwrap().clone()
    }
    fn set_counter(&mut self, cache_id: &str, key: &str, value: i64) {
        self.counters.get_mut(cache_id).unwrap().insert(key.to_string(), value);
    }
    fn add_to_counter(&mut self, cache_id: &str, key: &str, delta: i64) -> i64 {
        let m = self.counters.get_mut(cache_id).unwrap();
        let next = m.get(key).copied().unwrap_or(0) + delta;
        m.insert(key.to_string(), next);
        next
    }
    fn get_counter(&self, cache_id: &str, key: &str) -> Option<i64> {
        self.counters.get(cache_id).unwrap().get(key).copied()
    }
    fn counter_keys(&self, cache_id: &str) -> Vec<String> {
        self.counters.get(cache_id).unwrap().keys().cloned().collect()
    }
}

// ----------------------------------------------------------------------- report

#[derive(Serialize)]
struct ReportConfig {
    transport: String,
    #[serde(rename = "kvCacheCount")]
    kv_cache_count: usize,
    #[serde(rename = "counterCacheCount")]
    counter_cache_count: usize,
    #[serde(rename = "keySpace")]
    key_space: usize,
    #[serde(rename = "valueSizeBytes")]
    value_size_bytes: usize,
    #[serde(rename = "verifyEvery")]
    verify_every: u64,
    #[serde(rename = "opTimeoutSeconds")]
    op_timeout_seconds: u64,
}

#[derive(Serialize)]
struct HeapSample {
    #[serde(rename = "elapsedMillis")]
    elapsed_millis: u64,
    #[serde(rename = "usedHeapBytes")]
    used_heap_bytes: u64,
}

#[derive(Serialize)]
struct ReportJson {
    seed: u64,
    #[serde(rename = "durationSeconds")]
    duration_seconds: u64,
    config: ReportConfig,
    #[serde(rename = "startMillis")]
    start_millis: u64,
    #[serde(rename = "endMillis")]
    end_millis: u64,
    #[serde(rename = "totalOps")]
    total_ops: u64,
    #[serde(rename = "opCounts")]
    op_counts: BTreeMap<String, u64>,
    #[serde(rename = "heapSamples")]
    heap_samples: Vec<HeapSample>,
}

/// Accumulates per-operation counts and periodic process-memory samples, and writes a JSON report.
/// The memory samples are the primary leak signal: with a bounded key space, resident memory that
/// trends upward with wall-clock time (not with the data set) is a genuine client-side leak. Rust has
/// no runtime heap query, so we sample resident set size (RSS) from `/proc/self/statm`.
struct SoakReport {
    seed: u64,
    duration_seconds: u64,
    config: ReportConfig,
    start: Instant,
    start_millis: u64,
    op_counts: BTreeMap<String, u64>,
    total: u64,
    samples: Vec<(u64, u64)>,
}

impl SoakReport {
    fn new(cfg: &SoakConfig) -> Self {
        Self {
            seed: cfg.seed,
            duration_seconds: cfg.duration_seconds,
            config: cfg.to_report_config(),
            start: Instant::now(),
            start_millis: epoch_millis(),
            op_counts: BTreeMap::new(),
            total: 0,
            samples: Vec::new(),
        }
    }

    fn record_count(&mut self, op: &str) {
        *self.op_counts.entry(op.to_string()).or_insert(0) += 1;
        self.total += 1;
    }

    fn total(&self) -> u64 {
        self.total
    }

    fn sample_heap(&mut self) {
        self.samples.push((self.start.elapsed().as_millis() as u64, rss_bytes()));
    }

    fn progress_line(&self) -> String {
        let used = self.samples.last().map(|s| s.1).unwrap_or(0);
        format!(
            "ops={} elapsed={}s rssUsedMB={}",
            self.total,
            self.start.elapsed().as_secs(),
            used / (1024 * 1024)
        )
    }

    fn write_to(mut self, path: &str) -> Result<(), BoxErr> {
        self.sample_heap();
        let root = ReportJson {
            seed: self.seed,
            duration_seconds: self.duration_seconds,
            config: self.config,
            start_millis: self.start_millis,
            end_millis: epoch_millis(),
            total_ops: self.total,
            op_counts: self.op_counts,
            heap_samples: self
                .samples
                .iter()
                .map(|(e, u)| HeapSample { elapsed_millis: *e, used_heap_bytes: *u })
                .collect(),
        };
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(&root)?)?;
        Ok(())
    }
}

fn epoch_millis() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}

/// Resident set size of this process in bytes, read from `/proc/self/statm` (Linux CI runner).
fn rss_bytes() -> u64 {
    let page = 4096u64;
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1).map(|t| t.to_string()))
        .and_then(|t| t.parse::<u64>().ok())
        .map(|pages| pages * page)
        .unwrap_or(0)
}

// ----------------------------------------------------------------------- transport facade

const SUCCESS: &str = "SUCCESS";

/// Transport-neutral facade over the client libraries, so a single workload can drive the HTTP, bidi
/// or gateway client. All three expose identical synchronous op methods returning the same model
/// types, so each implementation is a thin wrapper.
trait SoakClient {
    fn name(&self) -> &str;
    fn create_kv_cache(&self, cache_id: &str) -> Result<(), BoxErr>;
    fn create_counter_cache(&self, cache_id: &str) -> Result<(), BoxErr>;
    fn put(&self, cache_id: &str, key: &str, value: &str) -> Result<String, BoxErr>;
    fn get(&self, cache_id: &str, key: &str) -> Result<Option<String>, BoxErr>;
    fn remove(&self, cache_id: &str, key: &str) -> Result<String, BoxErr>;
    fn clear(&self, cache_id: &str) -> Result<String, BoxErr>;
    fn delete_cache(&self, cache_id: &str) -> Result<String, BoxErr>;
    fn list_kv(&self, cache_id: &str) -> Result<BTreeMap<String, String>, BoxErr>;
    fn put_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<String, BoxErr>;
    fn increment_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<i64, BoxErr>;
    fn decrement_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<i64, BoxErr>;
    fn set_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<i64, BoxErr>;
    fn get_counter(&self, cache_id: &str, key: &str) -> Result<Option<i64>, BoxErr>;
    fn close(self: Box<Self>);
}

/// SoakClient over the HTTP/REST `AeronCacheClient`.
struct HttpSoakClient {
    client: AeronCacheClient,
}

impl SoakClient for HttpSoakClient {
    fn name(&self) -> &str {
        "http"
    }
    fn create_kv_cache(&self, cache_id: &str) -> Result<(), BoxErr> {
        self.client.create_cache(cache_id).map(|_| ())
    }
    fn create_counter_cache(&self, cache_id: &str) -> Result<(), BoxErr> {
        self.client.create_counter_cache(cache_id).map(|_| ())
    }
    fn put(&self, cache_id: &str, key: &str, value: &str) -> Result<String, BoxErr> {
        Ok(self.client.put_item(cache_id, key, value)?.operation_status)
    }
    fn get(&self, cache_id: &str, key: &str) -> Result<Option<String>, BoxErr> {
        let r = self.client.get_item(cache_id, key)?;
        Ok(if r.operation_status == SUCCESS { Some(r.value) } else { None })
    }
    fn remove(&self, cache_id: &str, key: &str) -> Result<String, BoxErr> {
        Ok(self.client.delete_item(cache_id, key)?.operation_status)
    }
    fn clear(&self, cache_id: &str) -> Result<String, BoxErr> {
        Ok(self.client.clear_cache(cache_id)?.operation_status)
    }
    fn delete_cache(&self, cache_id: &str) -> Result<String, BoxErr> {
        Ok(self.client.delete_cache(cache_id)?.operation_status)
    }
    fn list_kv(&self, cache_id: &str) -> Result<BTreeMap<String, String>, BoxErr> {
        Ok(self
            .client
            .get_cache_items(cache_id)?
            .items
            .into_iter()
            .map(|i| (i.key, i.value))
            .collect())
    }
    fn put_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<String, BoxErr> {
        Ok(self.client.put_counter(cache_id, key, value)?.operation_status)
    }
    fn increment_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<i64, BoxErr> {
        Ok(self.client.increment_counter(cache_id, key, amount)?.value)
    }
    fn decrement_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<i64, BoxErr> {
        Ok(self.client.decrement_counter(cache_id, key, amount)?.value)
    }
    fn set_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<i64, BoxErr> {
        Ok(self.client.set_counter(cache_id, key, value)?.value)
    }
    fn get_counter(&self, cache_id: &str, key: &str) -> Result<Option<i64>, BoxErr> {
        let r = self.client.get_counter(cache_id, key)?;
        Ok(if r.operation_status == SUCCESS { Some(r.value) } else { None })
    }
    fn close(self: Box<Self>) {
        // AeronCacheClient holds no long-lived connection to close.
    }
}

/// SoakClient over the bidirectional-WebSocket `AeronBidiClient`.
struct BidiSoakClient {
    client: AeronBidiClient,
}

impl SoakClient for BidiSoakClient {
    fn name(&self) -> &str {
        "bidi"
    }
    fn create_kv_cache(&self, cache_id: &str) -> Result<(), BoxErr> {
        self.client.create_cache(cache_id).map(|_| ())
    }
    fn create_counter_cache(&self, cache_id: &str) -> Result<(), BoxErr> {
        self.client.create_counter_cache(cache_id).map(|_| ())
    }
    fn put(&self, cache_id: &str, key: &str, value: &str) -> Result<String, BoxErr> {
        Ok(self.client.put_item(cache_id, key, value)?.operation_status)
    }
    fn get(&self, cache_id: &str, key: &str) -> Result<Option<String>, BoxErr> {
        let r = self.client.get_item(cache_id, key)?;
        Ok(if r.operation_status == SUCCESS { Some(r.value) } else { None })
    }
    fn remove(&self, cache_id: &str, key: &str) -> Result<String, BoxErr> {
        Ok(self.client.delete_item(cache_id, key)?.operation_status)
    }
    fn clear(&self, cache_id: &str) -> Result<String, BoxErr> {
        Ok(self.client.clear_cache(cache_id)?.operation_status)
    }
    fn delete_cache(&self, cache_id: &str) -> Result<String, BoxErr> {
        Ok(self.client.delete_cache(cache_id)?.operation_status)
    }
    fn list_kv(&self, cache_id: &str) -> Result<BTreeMap<String, String>, BoxErr> {
        Ok(self
            .client
            .get_cache_items(cache_id)?
            .items
            .into_iter()
            .map(|i| (i.key, i.value))
            .collect())
    }
    fn put_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<String, BoxErr> {
        Ok(self.client.put_counter(cache_id, key, value)?.operation_status)
    }
    fn increment_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<i64, BoxErr> {
        Ok(self.client.increment_counter(cache_id, key, amount)?.value)
    }
    fn decrement_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<i64, BoxErr> {
        Ok(self.client.decrement_counter(cache_id, key, amount)?.value)
    }
    fn set_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<i64, BoxErr> {
        Ok(self.client.set_counter(cache_id, key, value)?.value)
    }
    fn get_counter(&self, cache_id: &str, key: &str) -> Result<Option<i64>, BoxErr> {
        let r = self.client.get_counter(cache_id, key)?;
        Ok(if r.operation_status == SUCCESS { Some(r.value) } else { None })
    }
    fn close(self: Box<Self>) {
        // AeronBidiClient closes its WebSocket via its Drop impl when this box is dropped.
    }
}

/// SoakClient over the Aeron-gateway `AeronGatewayClient`. Owns the embedded media driver it connects
/// through (mirroring the gateway integration tests); both are dropped on `close`.
struct GatewaySoakClient {
    client: AeronGatewayClient,
    _driver: EmbeddedDriver,
}

impl SoakClient for GatewaySoakClient {
    fn name(&self) -> &str {
        "gateway"
    }
    fn create_kv_cache(&self, cache_id: &str) -> Result<(), BoxErr> {
        self.client.create_cache(cache_id).map(|_| ())
    }
    fn create_counter_cache(&self, cache_id: &str) -> Result<(), BoxErr> {
        self.client.create_counter_cache(cache_id).map(|_| ())
    }
    fn put(&self, cache_id: &str, key: &str, value: &str) -> Result<String, BoxErr> {
        Ok(self.client.put_item(cache_id, key, value)?.operation_status)
    }
    fn get(&self, cache_id: &str, key: &str) -> Result<Option<String>, BoxErr> {
        let r = self.client.get_item(cache_id, key)?;
        Ok(if r.operation_status == SUCCESS { Some(r.value) } else { None })
    }
    fn remove(&self, cache_id: &str, key: &str) -> Result<String, BoxErr> {
        Ok(self.client.delete_item(cache_id, key)?.operation_status)
    }
    fn clear(&self, cache_id: &str) -> Result<String, BoxErr> {
        Ok(self.client.clear_cache(cache_id)?.operation_status)
    }
    fn delete_cache(&self, cache_id: &str) -> Result<String, BoxErr> {
        Ok(self.client.delete_cache(cache_id)?.operation_status)
    }
    fn list_kv(&self, cache_id: &str) -> Result<BTreeMap<String, String>, BoxErr> {
        Ok(self
            .client
            .get_cache_items(cache_id)?
            .items
            .into_iter()
            .map(|i| (i.key, i.value))
            .collect())
    }
    fn put_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<String, BoxErr> {
        Ok(self.client.put_counter(cache_id, key, value)?.operation_status)
    }
    fn increment_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<i64, BoxErr> {
        Ok(self.client.increment_counter(cache_id, key, amount)?.value)
    }
    fn decrement_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<i64, BoxErr> {
        Ok(self.client.decrement_counter(cache_id, key, amount)?.value)
    }
    fn set_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<i64, BoxErr> {
        Ok(self.client.set_counter(cache_id, key, value)?.value)
    }
    fn get_counter(&self, cache_id: &str, key: &str) -> Result<Option<i64>, BoxErr> {
        let r = self.client.get_counter(cache_id, key)?;
        Ok(if r.operation_status == SUCCESS { Some(r.value) } else { None })
    }
    fn close(self: Box<Self>) {
        // AeronGatewayClient and the embedded media driver are dropped here.
    }
}

// ----------------------------------------------------------------------- workload

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpType {
    KvPut,
    KvGet,
    KvRemove,
    KvClear,
    KvDeleteRecreate,
    CounterPut,
    CounterInc,
    CounterDec,
    CounterSet,
    CounterGet,
}

impl OpType {
    fn name(self) -> &'static str {
        match self {
            OpType::KvPut => "KV_PUT",
            OpType::KvGet => "KV_GET",
            OpType::KvRemove => "KV_REMOVE",
            OpType::KvClear => "KV_CLEAR",
            OpType::KvDeleteRecreate => "KV_DELETE_RECREATE",
            OpType::CounterPut => "COUNTER_PUT",
            OpType::CounterInc => "COUNTER_INC",
            OpType::CounterDec => "COUNTER_DEC",
            OpType::CounterSet => "COUNTER_SET",
            OpType::CounterGet => "COUNTER_GET",
        }
    }
}

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const PROGRESS_EVERY: u64 = 5000;

struct SoakWorkload {
    client: Box<dyn SoakClient>,
    cfg: SoakConfig,
    report: SoakReport,
    oracle: SoakOracle,
    rnd: SoakRandom,
    op_timeout: Duration,
    op_table: Vec<OpType>,
    kv_cache_ids: Vec<String>,
    counter_cache_ids: Vec<String>,
}

impl SoakWorkload {
    fn new(client: Box<dyn SoakClient>, cfg: SoakConfig) -> Self {
        let report = SoakReport::new(&cfg);
        let rnd = SoakRandom::new(cfg.seed);
        let op_timeout = Duration::from_secs(cfg.op_timeout_seconds);
        let op_table = build_op_table(&cfg);
        Self {
            client,
            cfg,
            report,
            oracle: SoakOracle::default(),
            rnd,
            op_timeout,
            op_table,
            kv_cache_ids: Vec::new(),
            counter_cache_ids: Vec::new(),
        }
    }

    fn run(mut self) -> Result<SoakReport, BoxErr> {
        println!(
            "[soak] starting transport={} ({}) durationSeconds={} seed={} keySpace={} valueSizeBytes={}",
            self.cfg.transport,
            self.client.name(),
            self.cfg.duration_seconds,
            self.cfg.seed,
            self.cfg.key_space,
            self.cfg.value_size_bytes
        );
        if let Err(e) = self.drive() {
            // Make every failure reproducible: the seed replays the exact op sequence.
            return Err(format!(
                "Soak failed (seed={}, transport={}, opsSoFar={}): {}",
                self.cfg.seed,
                self.cfg.transport,
                self.report.total(),
                e
            )
            .into());
        }
        // Explicitly tear the client down (mirrors the Java soak's try-with-resources close).
        let SoakWorkload { client, report, .. } = self;
        client.close();
        Ok(report)
    }

    fn drive(&mut self) -> Result<(), BoxErr> {
        self.setup_caches()?;
        let deadline = Instant::now() + Duration::from_secs(self.cfg.duration_seconds);
        while Instant::now() < deadline {
            let op = self.op_table[self.rnd.next_int(self.op_table.len())];
            let applied = self.execute(op)?;
            self.report.record_count(applied.name());

            let ops = self.report.total();
            if ops % self.cfg.verify_every == 0 {
                self.reconcile_all()?;
            }
            if ops % PROGRESS_EVERY == 0 {
                self.report.sample_heap();
                println!("[soak] {}", self.report.progress_line());
            }
        }
        self.reconcile_all()?; // end on a verified-consistent state
        println!("[soak] complete: {}", self.report.progress_line());
        Ok(())
    }

    fn setup_caches(&mut self) -> Result<(), BoxErr> {
        for i in 0..self.cfg.kv_cache_count {
            let id = format!("soak-{}-kv-{}", self.cfg.transport, i);
            self.client.create_kv_cache(&id)?;
            self.oracle.register_kv_cache(&id);
            self.kv_cache_ids.push(id);
        }
        for i in 0..self.cfg.counter_cache_count {
            let id = format!("soak-{}-ctr-{}", self.cfg.transport, i);
            self.client.create_counter_cache(&id)?;
            self.oracle.register_counter_cache(&id);
            self.counter_cache_ids.push(id);
        }
        Ok(())
    }

    /// Returns the op actually applied (an op may fall back when its precondition is unmet).
    fn execute(&mut self, op: OpType) -> Result<OpType, BoxErr> {
        match op {
            OpType::KvPut => {
                let cache = self.random_kv();
                let key = self.random_key();
                let val = self.random_value();
                let status = self.timed(|c| c.put(&cache, &key, &val))?;
                check(status == SUCCESS, || format!("put !SUCCESS {}/{}", cache, key))?;
                self.oracle.put_kv(&cache, &key, &val);
                Ok(OpType::KvPut)
            }
            OpType::KvGet => {
                let cache = self.random_kv();
                let key = self.random_key();
                let actual = self.timed(|c| c.get(&cache, &key))?;
                let expected = self.oracle.get_kv(&cache, &key);
                check(actual == expected, || {
                    format!("kv get mismatch {}/{} expected={:?} actual={:?}", cache, key, expected, actual)
                })?;
                Ok(OpType::KvGet)
            }
            OpType::KvRemove => {
                let cache = self.random_kv();
                let keys = self.oracle.kv_keys(&cache);
                if keys.is_empty() {
                    return self.execute(OpType::KvPut);
                }
                let key = keys[self.rnd.next_int(keys.len())].clone();
                let status = self.timed(|c| c.remove(&cache, &key))?;
                check(status == SUCCESS, || format!("remove !SUCCESS {}/{}", cache, key))?;
                self.oracle.remove_kv(&cache, &key);
                Ok(OpType::KvRemove)
            }
            OpType::KvClear => {
                let cache = self.random_kv();
                let status = self.timed(|c| c.clear(&cache))?;
                check(status == SUCCESS, || format!("clear !SUCCESS {}", cache))?;
                self.oracle.clear_kv(&cache);
                Ok(OpType::KvClear)
            }
            OpType::KvDeleteRecreate => {
                let cache = self.random_kv();
                let status = self.timed(|c| c.delete_cache(&cache))?;
                check(status == SUCCESS, || format!("deleteCache !SUCCESS {}", cache))?;
                self.timed(|c| c.create_kv_cache(&cache))?;
                self.oracle.clear_kv(&cache);
                Ok(OpType::KvDeleteRecreate)
            }
            OpType::CounterPut => {
                // put_counter is the counter creator (set/inc/dec require the key to already exist).
                let cache = self.random_counter();
                let key = self.random_key();
                let value = self.rnd.next_i64(1000);
                let status = self.timed(|c| c.put_counter(&cache, &key, value))?;
                check(status == SUCCESS, || format!("putCounter !SUCCESS {}/{}", cache, key))?;
                self.oracle.set_counter(&cache, &key, value);
                self.verify_counter(&cache, &key, Some(value))?;
                Ok(OpType::CounterPut)
            }
            OpType::CounterInc | OpType::CounterDec => {
                let cache = self.random_counter();
                let key = match self.pick_present_counter_key(&cache) {
                    Some(k) => k,
                    None => return self.execute(OpType::CounterPut), // seed a counter first
                };
                let amount = 1 + self.rnd.next_i64(10);
                let delta = if op == OpType::CounterInc { amount } else { -amount };
                if op == OpType::CounterInc {
                    self.timed(|c| c.increment_counter(&cache, &key, amount))?;
                } else {
                    self.timed(|c| c.decrement_counter(&cache, &key, amount))?;
                }
                let expected = self.oracle.add_to_counter(&cache, &key, delta);
                // Verify via read-back rather than the mutation's echoed value (varies by transport).
                self.verify_counter(&cache, &key, Some(expected))?;
                Ok(op)
            }
            OpType::CounterSet => {
                let cache = self.random_counter();
                let key = match self.pick_present_counter_key(&cache) {
                    Some(k) => k,
                    None => return self.execute(OpType::CounterPut),
                };
                let value = self.rnd.next_i64(1000);
                self.timed(|c| c.set_counter(&cache, &key, value))?;
                self.oracle.set_counter(&cache, &key, value);
                self.verify_counter(&cache, &key, Some(value))?;
                Ok(OpType::CounterSet)
            }
            OpType::CounterGet => {
                let cache = self.random_counter();
                let key = match self.pick_present_counter_key(&cache) {
                    Some(k) => k,
                    None => return self.execute(OpType::CounterPut),
                };
                let expected = self.oracle.get_counter(&cache, &key);
                self.verify_counter(&cache, &key, expected)?;
                Ok(OpType::CounterGet)
            }
        }
    }

    fn reconcile_all(&mut self) -> Result<(), BoxErr> {
        let kv_ids = self.kv_cache_ids.clone();
        for cache in &kv_ids {
            let expected = self.oracle.kv_snapshot(cache);
            let actual = self.client.list_kv(cache)?;
            check(actual == expected, || {
                format!(
                    "kv reconcile mismatch {} expectedSize={} actualSize={} diff={}",
                    cache,
                    expected.len(),
                    actual.len(),
                    first_diff(&expected, &actual)
                )
            })?;
        }
        // Counters reconcile per key over the bounded key space (inline inc/dec/set verification
        // already covers value drift continuously).
        let ctr_ids = self.counter_cache_ids.clone();
        for cache in &ctr_ids {
            for key in self.oracle.counter_keys(cache) {
                let expected = self.oracle.get_counter(cache, &key);
                let actual = self.client.get_counter(cache, &key)?;
                check(actual == expected, || {
                    format!("counter reconcile mismatch {}/{} expected={:?} actual={:?}", cache, key, expected, actual)
                })?;
            }
        }
        Ok(())
    }

    fn random_kv(&mut self) -> String {
        let i = self.rnd.next_int(self.kv_cache_ids.len());
        self.kv_cache_ids[i].clone()
    }

    fn random_counter(&mut self) -> String {
        let i = self.rnd.next_int(self.counter_cache_ids.len());
        self.counter_cache_ids[i].clone()
    }

    fn random_key(&mut self) -> String {
        format!("k{}", self.rnd.next_int(self.cfg.key_space))
    }

    fn random_value(&mut self) -> String {
        let mut s = String::with_capacity(self.cfg.value_size_bytes);
        for _ in 0..self.cfg.value_size_bytes {
            s.push(ALPHABET[self.rnd.next_int(ALPHABET.len())] as char);
        }
        s
    }

    fn pick_present_counter_key(&mut self, cache: &str) -> Option<String> {
        let keys = self.oracle.counter_keys(cache);
        if keys.is_empty() {
            None
        } else {
            Some(keys[self.rnd.next_int(keys.len())].clone())
        }
    }

    fn verify_counter(&mut self, cache: &str, key: &str, expected: Option<i64>) -> Result<(), BoxErr> {
        let actual = self.timed(|c| c.get_counter(cache, key))?;
        check(actual == expected, || {
            format!("counter mismatch {}/{} expected={:?} actual={:?}", cache, key, expected, actual)
        })
    }

    /// Run a client op and fail if it exceeds the per-op timeout (a stalled op is a soak finding).
    fn timed<T>(&self, op: impl FnOnce(&dyn SoakClient) -> Result<T, BoxErr>) -> Result<T, BoxErr> {
        let start = Instant::now();
        let result = op(&*self.client)?;
        let elapsed = start.elapsed();
        if elapsed > self.op_timeout {
            return Err(format!(
                "operation exceeded {}s ({}ms)",
                self.cfg.op_timeout_seconds,
                elapsed.as_millis()
            )
            .into());
        }
        Ok(result)
    }
}

fn build_op_table(cfg: &SoakConfig) -> Vec<OpType> {
    let mut table = Vec::new();
    let add = |op: OpType, weight: usize, t: &mut Vec<OpType>| {
        for _ in 0..weight {
            t.push(op);
        }
    };
    if cfg.kv_cache_count > 0 {
        add(OpType::KvPut, 5, &mut table);
        add(OpType::KvGet, 5, &mut table);
        add(OpType::KvRemove, 2, &mut table);
        add(OpType::KvClear, 1, &mut table);
        add(OpType::KvDeleteRecreate, 1, &mut table);
    }
    if cfg.counter_cache_count > 0 {
        add(OpType::CounterPut, 3, &mut table);
        add(OpType::CounterInc, 4, &mut table);
        add(OpType::CounterDec, 2, &mut table);
        add(OpType::CounterSet, 2, &mut table);
        add(OpType::CounterGet, 4, &mut table);
    }
    assert!(!table.is_empty(), "soak needs at least one kv or counter cache");
    table
}

fn check(condition: bool, message: impl FnOnce() -> String) -> Result<(), BoxErr> {
    if condition {
        Ok(())
    } else {
        Err(message().into())
    }
}

fn first_diff(expected: &BTreeMap<String, String>, actual: &BTreeMap<String, String>) -> String {
    for (k, v) in expected {
        if actual.get(k) != Some(v) {
            return format!("key={} expected={} actual={:?}", k, v, actual.get(k));
        }
    }
    for k in actual.keys() {
        if !expected.contains_key(k) {
            return format!("spurious key={} actual={:?}", k, actual.get(k));
        }
    }
    "(none)".to_string()
}

// ----------------------------------------------------------------------- entry point

fn soak_enabled() -> bool {
    std::env::var("SOAK_ENABLED").map(|v| v == "true").unwrap_or(false)
}

fn build_client(cfg: &SoakConfig) -> Result<Option<Box<dyn SoakClient>>, BoxErr> {
    match cfg.transport.as_str() {
        "http" | "bidi" => {
            let base_url = std::env::var("AERON_CACHE_BASE_URL").unwrap_or_default();
            let mut ws_url = std::env::var("AERON_CACHE_WS_URL").unwrap_or_default();
            if !base_url.is_empty() && ws_url.is_empty() {
                ws_url = base_url.replace("http://", "ws://").replace("https://", "wss://");
            }
            if base_url.is_empty() || ws_url.is_empty() {
                eprintln!("[soak] skipped: AERON_CACHE_BASE_URL / AERON_CACHE_WS_URL not set (no backend)");
                return Ok(None);
            }
            if cfg.transport == "http" {
                Ok(Some(Box::new(HttpSoakClient {
                    client: AeronCacheClient::new(base_url, ws_url),
                })))
            } else {
                Ok(Some(Box::new(BidiSoakClient {
                    client: AeronBidiClient::connect(&ws_url)?,
                })))
            }
        }
        "gateway" => {
            let host = std::env::var("AERON_GATEWAY_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
            let driver = EmbeddedDriver::launch().map_err(|e| format!("failed to launch embedded media driver: {e}"))?;
            let client = AeronGatewayClient::connect(driver.dir(), &host)?
                .with_request_timeout(Duration::from_secs(cfg.op_timeout_seconds));
            if !client.await_connected(Duration::from_secs(10)) {
                return Err(format!("gateway did not connect at host {host}").into());
            }
            Ok(Some(Box::new(GatewaySoakClient { client, _driver: driver })))
        }
        other => Err(format!("unknown SOAK_TRANSPORT: {other} (expected http|bidi|gateway)").into()),
    }
}

#[test]
fn soak() {
    if !soak_enabled() {
        eprintln!("Soak disabled: set SOAK_ENABLED=true (run via the Soak Rust workflow)");
        return;
    }

    let cfg = SoakConfig::from_env();
    let client = match build_client(&cfg).expect("failed to build soak client") {
        Some(c) => c,
        None => return, // no backend configured for http/bidi -> skip (green)
    };

    let transport = cfg.transport.clone();
    let workload = SoakWorkload::new(client, cfg);
    // `run` takes ownership of the workload (and its client); extract the report, then close.
    let report = workload.run().expect("soak run failed");
    report
        .write_to(&format!("reports/soak/soak-{transport}.json"))
        .expect("failed to write soak report");
}
