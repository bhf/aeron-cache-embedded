# Aeron Cache Embedded Clients

![img.png](https://raw.githubusercontent.com/bhf/aeron-cache/refs/heads/main/docs/images/header.png)

[![Java CI](https://github.com/bhf/aeron-cache-embedded/actions/workflows/java-ci.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/java-ci.yml)
[![TypeScript CI](https://github.com/bhf/aeron-cache-embedded/actions/workflows/typescript-ci.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/typescript-ci.yml)
[![Python CI](https://github.com/bhf/aeron-cache-embedded/actions/workflows/python-ci.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/python-ci.yml)
[![Rust CI](https://github.com/bhf/aeron-cache-embedded/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/rust-ci.yml)
[![Monolith (macOS)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/monolith-macos.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/monolith-macos.yml)
[![Monolith (Windows)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/monolith-windows.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/monolith-windows.yml)
[![Soak Java](https://github.com/bhf/aeron-cache-embedded/actions/workflows/soak-java.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/soak-java.yml)
[![Soak TypeScript](https://github.com/bhf/aeron-cache-embedded/actions/workflows/soak-typescript.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/soak-typescript.yml)
[![Soak Python](https://github.com/bhf/aeron-cache-embedded/actions/workflows/soak-python.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/soak-python.yml)
[![Soak Rust](https://github.com/bhf/aeron-cache-embedded/actions/workflows/soak-rust.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/soak-rust.yml)

Embedded cache client SDKs for Aeron Cache in multiple languages: Java, TypeScript, Python, and Rust, with minimal external dependencies.

The goal of these libraries is to provide an **"Embedded Cache"** mode across languages. In this mode, the client maintains a local copy of the cache data which is kept in sync with the server via streaming updates, allowing for fast local reads with no network round-trip.

All four libraries cover the full cache **and** counter surface — CRUD, timed (TTL) entries, JSON deep-merge patch, cancel-removal, bulk operations, live subscriptions (including keyed and patch-mode), and inspection/management operations (list caches, stats, entries, and pending TTL timers).

### Transports

The same high-level API is offered over multiple transports:

| Transport | Endpoint | Languages | Notes |
|:----------|:---------|:----------|:------|
| **HTTP + WebSocket** | `:7070` (HTTP) / `:7071` (WS) | Java, TypeScript, Python, Rust | Commands over HTTP; streaming subscriptions over WebSocket. |
| **Bidirectional WebSocket** | `/api/ws/v1/bidi` | Java, TypeScript, Python, Rust | Full command + subscription surface multiplexed over a single JSON WebSocket connection (`AeronBidiClient`). |
| **Aeron SBE Gateway** | UDP `:7075` / `:7076` | Java, Rust | Commands and streaming over Aeron UDP using an SBE wire protocol (`AeronGatewayClient`). |

## Table of Contents
- [Functionality and Samples By Language](#functionality-and-samples-by-language)
- [Installation](#installation)
- [Example Usage](#example-usage)
- [Near Cache](#near-cache)
- [Running Samples](#running-samples)
- [Features](#features)
- [Soak Testing](#soak-testing)
- [Project Structure](#project-structure)
- [Adding New Languages](#adding-new-languages)

## Functionality and Samples By Language

| Language   | Sync | Async | Streaming (Embedded) | Bulk | Bidi WebSocket | Aeron Gateway |
|:-----------|:----:|:-----:|:--------------------:|:----:|:--------------:|:-------------:|
| Java       |  ✅  |  ✅   |          ✅          |  ✅  |       ✅       |      ✅       |
| TypeScript |  NA  |  ✅   |          ✅          |  ✅  |       ✅       |      NA       |
| Python     |  ✅  |  ✅   |          ✅          |  ✅  |       ✅       |      NA       |
| Rust       |  ✅  |  ✅   |          ✅          |  ✅  |       ✅       |      ✅       |

The Aeron SBE gateway transport is available in Java and Rust only; the bidirectional WebSocket transport is the JSON analogue for TypeScript and Python (and is available in Java and Rust too).

[Back to top](#aeron-cache-embedded-clients)

## Installation

### Java (GitHub Packages)
Add the GitHub Packages repository and dependency to your build file:
```kotlin
repositories {
    maven {
        url = uri("https://maven.pkg.github.com/bhf/aeron-cache-embedded")
    }
}
dependencies {
    implementation("com.aeron.cache:aeron-cache-embedded-client:1.0.0")
}
```

### TypeScript (GitHub Packages npm registry)
Configure your `.npmrc` to use GitHub Packages for the `@bhf` scope, then install:
```bash
npm install @bhf/aeron-cache-embedded-client
```

### Python
You can install the Python client directly from this repository via git:
```bash
pip install "git+https://github.com/bhf/aeron-cache-embedded.git@py-v1.0.0#subdirectory=libraries/python"
```
Alternatively, `.whl` and `.tar.gz` files are attached to the [GitHub Releases](https://github.com/bhf/aeron-cache-embedded/releases).

### Rust
You can add the Rust client as a git dependency in your `Cargo.toml`:
```toml
[dependencies]
aeron-cache-embedded-client = { git = "https://github.com/bhf/aeron-cache-embedded", tag = "rust-v1.0.0" }
```
Alternatively, `.crate` archives are available on the [GitHub Releases](https://github.com/bhf/aeron-cache-embedded/releases) page.

[Back to top](#aeron-cache-embedded-clients)

## Example Usage

### Java

```java
var baseUrl = "http://localhost:7070";
var wsUrl = "ws://localhost:7071";
AeronCacheClient client = new AeronCacheClient(baseUrl, wsUrl);
client.createCache("sample-cache");

// Embedded cache — writes go to the server, reads are served from the local mirror.
EmbeddedAeronCache cache = client.getCache("sample-cache");
cache.put("stay", "tuned");
```

### TypeScript

```typescript
const baseUrl = "http://localhost:7070";
const wsUrl = "ws://localhost:7071";
const cacheClient = new AeronCacheClient(baseUrl, wsUrl);

await cacheClient.createCache('sample-cache');

const cache = new EmbeddedAeronCache(cacheClient, 'sample-cache');
await cache.put("stay", "tuned");
```

### Counters

In addition to string caches, all clients support **counter caches** whose values are 64-bit integers. Counter caches expose the same lifecycle operations (create / put / timed put / get / delete / subscribe) plus counter-specific `increment`, `decrement`, and `set` operations, and an `EmbeddedCounterCache` that shadows values locally over the stream.

```typescript
// TypeScript
await cacheClient.createCounterCache('counter-cache');
const counters = cacheClient.getCounterCache('counter-cache');
await counters.put('requests', 10);
await counters.increment('requests', 5); // -> 15
await counters.decrement('requests', 3); // -> 12
await counters.set('requests', 100);     // -> 100
```

### Patch, cancel-removal, and inspection

```typescript
// JSON deep-merge patch: merge {"b":2} into the stored value without replacing it.
await cacheClient.putItem('sample-cache', 'doc', '{"a":1}');
await cacheClient.patchItem('sample-cache', 'doc', '{"b":2}'); // -> {"a":1,"b":2}

// Timed entries schedule a TTL removal; cancel it to keep the entry.
await cacheClient.putTimedItem('sample-cache', 'session', 'active', 60000);
await cacheClient.cancelItemRemoval('sample-cache', 'session');

// Inspection & management.
await cacheClient.getCaches();   // list caches
await cacheClient.getStats();    // aggregate statistics
await cacheClient.getTimers();   // all pending TTL removal timers (cache + counter)
```

### Bulk operations

Submit a batch of cache and/or counter operations in a single request. A batch may freely mix regular-cache and counter operations; each operation carries its own `requestId`, echoed on the matching per-operation response.

```typescript
const response = await cacheClient.bulkOps({
  requestId: 'batch-1',
  operations: [
    { operationType: 'ADD_ITEM', requestId: 'op-1', cacheId: 'sample-cache', key: 'k1', value: 'v1' },
    { operationType: 'INCREMENT_COUNTER', requestId: 'op-2', cacheId: 'counter-cache', key: 'requests', counterValue: 5 },
    { operationType: 'GET_ITEM', requestId: 'op-3', cacheId: 'sample-cache', key: 'k1' },
  ],
});
```

### Bidirectional WebSocket

`AeronBidiClient` carries the full command surface — including bulk and `getTimers` — plus dynamic subscribe/unsubscribe over a single persistent WebSocket connection.

```typescript
const client = new AeronBidiClient('ws://localhost:7071');
await client.createCache('bidi-cache');
await client.putItem('bidi-cache', 'k', 'v');
const timers = await client.getTimers();
```

See the `samples/` directory for full, runnable code examples in all languages.

[Back to top](#aeron-cache-embedded-clients)

## Near Cache

The backend supports an optional **server-side near cache**: the interface process keeps a local, read-ahead copy of a cache (or counter cache) warm by subscribing to it, so reads are served locally instead of round-tripping to the cluster, falling back to the source cache on a miss. It is a **deployment option**, enabled on the backend by setting `NEAR_CACHE_ENABLED=true` on the HTTP, WebSocket, and/or SSE interface.

It is designed to be **transparent to the client**: it lives on the **same URIs** you already use, so turning it on or off is a backend decision that requires **no client change and no change to your call sites**. These libraries need no new code to benefit from it — the table below is purely informational.

This is complementary to the **client-side** [Embedded Cache](#features) in these libraries: the embedded cache mirrors data in *your process* over the stream, while the near cache mirrors it in *the server's* interface process. Creation, puts, and deletes always go to the cluster over the regular endpoints, which the near cache picks up through its own subscription.

### How each transport benefits

| Path | What happens when `NEAR_CACHE_ENABLED=true` | Your code |
|:-----|:--------------------------------------------|:----------|
| **Streaming / Embedded Cache** | A **hydrating** subscription (`hydrate=true`) has its initial snapshot served from the warm near cache rather than a fresh cluster fetch. The embedded caches in all four libraries use this path already. | Unchanged |
| **HTTP reads** | The regular read methods — `getItem` / `getCacheItems` / `getCounter` / `getCounterItems` (and their language equivalents) — are served from the near cache on their usual `/api/v1/cache/` and `/api/v1/counters/` URIs. | Unchanged |

Because near caching is read-ahead and eventually consistent, a read of an already-warm key returns the local copy, which can briefly lag a concurrent write until the update event lands — a deliberate trade-off you opt into at deploy time.

### Management endpoints

The only near-specific surface is management — listing the warm caches and stopping near caching for one — since these have no regular-endpoint equivalent. They are served under `/api/v1/near/...` on whichever interface has the near cache enabled:

| Method & URI | Purpose | Response shape |
|:-------------|:--------|:---------------|
| `GET /api/v1/near/caches` | List the caches currently held warm, with local item counts. | `CacheDetails[]` |
| `GET /api/v1/near/counters-caches` | List the counter caches currently held warm. | `CacheDetails[]` |
| `DELETE /api/v1/near/cache/<cacheId>` | Stop near caching a cache: drop the subscription and the local copy (idempotent). | `StopNearCacheResponse` |
| `DELETE /api/v1/near/counters/<cacheId>` | Stop near caching a counter cache. | `StopNearCacheResponse` |

These management operations are not currently surfaced as dedicated client methods; call them directly (a plain `GET`/`DELETE`) when you need them.

[Back to top](#aeron-cache-embedded-clients)

## Running Samples

Each language has a set of standalone samples under `samples/<language>/`: `sync-sample`, `async-sample`, `streaming-sample`, `bulk-sample`, `bidi-sample`, and (Java and Rust only) `aeron-sample`. Each sample's own README covers how to run it.

Two helper scripts run common subsets across all languages:

### Standard Samples (Sync/Async)
Runs the basic sync and async samples for all languages sequentially.
```bash
./run-all-samples.sh
```

### Streaming Samples (Embedded Cache)
Runs the streaming (Embedded Cache) samples for all languages in **parallel**, capturing the output from all languages into a single terminal window.
```bash
./run-streaming-samples.sh
```
*Note: Press Ctrl+C to stop all parallel streaming processes.*

The `bulk-sample`, `bidi-sample`, and `aeron-sample` demos are transport-specific and are run standalone per their own READMEs (the Aeron gateway sample additionally requires the gateway to be enabled on the backend — see [Transports](#transports)).

[Back to top](#aeron-cache-embedded-clients)

## Features

-   **Business Status Mapping**: Responses include an `operationStatus` field (e.g., `SUCCESS`, `CACHE_EXISTS`, `UNKNOWN_KEY`) to handle business logic without throwing transport-level exceptions for HTTP 400 errors.
-   **CRUD Operations**: Full support for Create, Get, Put, and Delete of items and caches.
-   **Timed Entries & Cancel-Removal**: Put entries with a TTL, and cancel a pending TTL removal (`cancelItemRemoval` / `cancel_item_removal`) to keep an entry alive.
-   **Patch (JSON deep-merge)**: `patchItem` / `patch_item` merges a JSON fragment into a stored value instead of replacing it.
-   **Counter Caches**: Dedicated `int64` counter caches with `increment`, `decrement`, `set`, and timed-put operations, plus an `EmbeddedCounterCache` that shadows counter values locally.
-   **Bulk Operations**: Submit multiple cache and counter operations in a single request via `bulkOps` / `bulk_ops`. Batches may mix cache and counter operations, each carrying a per-operation `requestId`; results are streamed back in request order.
-   **Subscriptions**: Live streaming updates, with **keyed subscriptions** (filter to specific keys) and **patch-mode subscriptions** (receive `PATCH_ITEM` deltas). A subscribe acknowledgement provides a barrier so callers can await a subscription going live.
-   **Inspection & Management**: List caches (`getCaches`), aggregate statistics (`getStats`), full entry snapshots (`getCacheItems` / `getCounterItems`), and all pending TTL removal timers (`getTimers`).
-   **Embedded Cache**: A specialized `EmbeddedAeronCache` / `EmbeddedCounterCache` that maintains a local shadowed copy of the cache data. It subscribes to the stream and applies updates (`ADD_ITEM`, `REMOVE_ITEM`, `DELETE_CACHE`, `CLEAR_CACHE`) to the local map automatically — over any transport. The string-valued `EmbeddedAeronCache` cannot merge `PATCH_ITEM` deltas into an opaque string, so it ignores them and rejects patch-mode subscriptions; use the embedded object cache for that.
-   **Embedded Object Cache**: An embedded cache whose values are structured JSON objects. It deep-merges `PATCH_ITEM` deltas into the stored object using RFC 7386 (JSON Merge Patch) semantics — nested objects merge recursively, scalars/arrays replace, and a `null` field deletes it — so patch-mode subscriptions (which stream only the changed fields) reconstruct the full object locally without losing untouched fields. Available in all four libraries: `EmbeddedObjectCache` (Java: Jackson `ObjectNode`, with `getLocalAs` to deserialize into a POJO; Python: `dict`; TypeScript: `Record<string, any>`) and `EmbeddedObjects` (Rust: `serde_json::Value`). Obtain one via `getObjectCache` / `get_object_cache` / `embedded_object_cache`.
-   **Near Cache (server-side)**: An optional read-ahead cache the backend keeps warm in its interface process (a deployment option, enabled with `NEAR_CACHE_ENABLED=true`). It is transparent to the client — reads and hydrating subscriptions are served from it on the same URIs you already use, so these libraries benefit with no code change. See [Near Cache](#near-cache).
-   **Multiple Transports**: HTTP+WebSocket, a bidirectional WebSocket (`AeronBidiClient`), and — in Java and Rust — an Aeron SBE UDP gateway (`AeronGatewayClient`). Batched responses (entries, stats, timers, bulk) are streamed in one or more batches terminated by an end-of-batch marker and reassembled by the client.
-   **Sync & Async**: APIs available in both synchronous (blocking) and asynchronous (non-blocking) styles where appropriate.

[Back to top](#aeron-cache-embedded-clients)

## Soak Testing

Each library has a long-running **soak** that hammers a real backend to surface correctness drift, stalls, and client-side memory leaks that only show up over time. The design is identical across languages (ported from the Java soak): a single-writer workload drives a weighted, seeded-random mix of key-value and counter operations over a bounded key space spread across a handful of caches, keeping an in-memory **oracle** in lock-step with every mutation. Every value is checked inline (read-backs on gets and counter inc/dec/set) and the full model is reconciled against the backend periodically and once at the end. Because the key space is bounded, client memory that trends up with *wall-clock time* (not with the data set) is a genuine leak — so heap/RSS is sampled throughout and written, alongside op counts and the resolved **seed**, to a JSON report (any finding is reproducible by re-running with the same seed).

Soaks are **manual-only** (`workflow_dispatch`), separate from the push/PR CI, and run per transport:

| Language   | Workflow                                                            | Transports                | Runner |
|:-----------|:-------------------------------------------------------------------|:--------------------------|:-------|
| Java       | `soak-java.yml`       | `http`, `bidi`, `gateway` | `./gradlew soakTest -Psoak.transport=<t>` |
| TypeScript | `soak-typescript.yml` | `http`, `bidi`            | `SOAK_TRANSPORT=<t> npm run soak` |
| Python     | `soak-python.yml`     | `http`, `bidi`            | `SOAK_TRANSPORT=<t> poetry run python -m soak` |
| Rust       | `soak-rust.yml`       | `http`, `bidi`, `gateway` | `SOAK_ENABLED=true SOAK_TRANSPORT=<t> cargo test --test soak -- --nocapture` |

All runs are time-bounded and fully parameterised. Java reads `-Psoak.*` project properties; the other languages read `SOAK_*` environment variables (`SOAK_DURATION_SECONDS`, `SOAK_SEED`, `SOAK_KV_CACHE_COUNT`, `SOAK_COUNTER_CACHE_COUNT`, `SOAK_KEY_SPACE`, `SOAK_VALUE_SIZE_BYTES`). HTTP/bidi read the backend URLs from `AERON_CACHE_BASE_URL` / `AERON_CACHE_WS_URL` and skip cleanly when they are unset; the Aeron gateway transport (Java and Rust) launches its own embedded media driver and connects over UDP. The `gateway` transport requires the backend's SBE gateway to be enabled (see [Transports](#transports)).

[Back to top](#aeron-cache-embedded-clients)

## Project Structure

-   `libraries/`: Core client libraries.
    -   `java/`: Java client library.
    -   `typescript/`: TypeScript/JavaScript client library.
    -   `python/`: Python client library.
    -   `rust/`: Rust client library.
-   `samples/`: Example applications using each of the clients.

[Back to top](#aeron-cache-embedded-clients)

## Adding New Languages

If you need to implement a client for a language not listed here, we provide **Agent Instructions** to help you scaffold a new library that follows the "Embedded Cache" pattern.

You can find the instructions here: [instructions/library-generation.instructions.md](instructions/library-generation.instructions.md)

These instructions cover:
- Tiers for low-level HTTP clients and high-level embedded abstractions.
- Automated synchronization logic via WebSockets.
- Data model requirements based on the [OpenAPI Specification](https://github.com/bhf/aeron-cache/blob/main/cache-http/openapi.yml).

[Back to top](#aeron-cache-embedded-clients)
