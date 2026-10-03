# Library Generation Instructions for Aeron Cache

You are tasked with creating a polyglot client library for **Aeron Cache**. Use these
instructions to scaffold the library in the user's language of choice so that it matches the
feature surface and conventions of the four existing clients (Java, TypeScript, Python, Rust).

> **Read the reference implementations first.** The authoritative source of truth is the existing
> code under `libraries/` and the samples under `samples/`. Pick the closest existing language
> (Python is the most readable reference for the full surface; Rust for the Aeron gateway) and
> mirror its structure, naming, and behaviour. This document describes *what* to build; the
> reference libraries show *how*.

## Core Concepts

The client follows a **tiered architecture**, and the same high-level API is offered over
**multiple transports**.

1.  **Low-Level Client (`AeronCacheClient`)**
    -   The entry point. Covers the full command surface (see [Operation Surface](#operation-surface)).
    -   Over the default transport it issues commands over **HTTP** and opens **WebSocket**
        connections for streaming subscriptions.
    -   Manages the base HTTP URL (e.g. `http://localhost:7070`) and WebSocket URL
        (e.g. `ws://localhost:7071`).
    -   Exposes both **synchronous** and **asynchronous** variants of operations where the
        language supports it (e.g. `put_item` / `put_item_async`). TypeScript is async-only.

2.  **High-Level Embedded Abstractions**
    -   Obtained from the client for a specific cache id. Each maintains a **local, in-memory
        mirror** of the cache that is kept in sync by subscribing to the server's update stream,
        so reads (`getLocal`) are served with no network round-trip.
    -   `EmbeddedAeronCache` (`client.getCache(cacheId)`) — string-valued cache.
    -   `EmbeddedCounterCache` (`client.getCounterCache(cacheId)`) — `int64` counter cache.
    -   `EmbeddedObjectCache` (`client.getObjectCache(cacheId)`) — structured JSON-object cache
        that deep-merges patch deltas (see [Embedded Object Cache](#embedded-object-cache)).

3.  **Transports** — the same high-level API, three wire protocols:

    | Transport | Endpoint | Scope | Notes |
    |:----------|:---------|:------|:------|
    | **HTTP + WebSocket** | `:7070` (HTTP) / `:7071` (WS) | Required | Commands over HTTP; streaming subscriptions over WebSocket. The baseline every client must implement. |
    | **Bidirectional WebSocket** (`AeronBidiClient`) | `ws://…/api/ws/v1/bidi` | Required | Full command + subscribe/unsubscribe surface multiplexed over a single JSON WebSocket connection. The portable full-surface transport. |
    | **Aeron SBE Gateway** (`AeronGatewayClient`) | UDP `:7075` / `:7076` (or IPC) | Optional | Commands and streaming over Aeron using an SBE wire protocol. Implement **only** where a mature Aeron client exists for the language (currently Java and Rust). |

## Operation Surface

Every transport exposes this surface (method names shown in `camelCase` / `snake_case`;
follow the target language's convention — see the reference clients for exact spellings).

### String caches
-   `createCache(cacheId)`
-   `putItem(cacheId, key, value)`
-   `putTimedItem(cacheId, key, value, ttlMillis)` — entry with a TTL removal timer.
-   `getItem(cacheId, key)` / `getCacheItems(cacheId)` — single item / full snapshot.
-   `patchItem(cacheId, key, jsonFragment)` — RFC 7386 JSON Merge Patch into a stored value.
-   `cancelItemRemoval(cacheId, key)` — cancel a pending TTL removal to keep the entry.
-   `deleteItem(cacheId, key)` / `clearCache(cacheId)` / `deleteCache(cacheId)`

### Counter caches (`int64` values)
-   `createCounterCache(cacheId)`
-   `putCounter(cacheId, key, value)` / `putTimedCounter(cacheId, key, value, ttlMillis)`
-   `getCounter(cacheId, key)` / `getCounterItems(cacheId)`
-   `incrementCounter(cacheId, key, amount)` / `decrementCounter(cacheId, key, amount)` / `setCounter(cacheId, key, value)`
-   `cancelCounterItemRemoval(cacheId, key)`
-   `deleteCounter(cacheId, key)` / `clearCounterCache(cacheId)` / `deleteCounterCache(cacheId)`

### Bulk
-   `bulkOps(request)` — submit a batch of operations in a single request. A batch may freely
    **mix** regular-cache and counter operations; each operation carries its own `requestId`,
    echoed on the matching per-operation response. See `BulkOperationType` for the full set of
    operation types.

### Inspection & management
-   `getCaches()` / `getCounterCaches()` — list caches.
-   `getStats()` / `getCounterStats()` — aggregate statistics.
-   `getTimers()` — all pending TTL removal timers (cache **and** counter).

### Subscriptions
-   `subscribe(cacheIds, onMessage, hydrate=false, keys=None, mode=None)` and
    `subscribeCounter(...)` — live streaming updates. Options:
    -   **hydrate**: on subscribe, replay the current contents as `ADD_ITEM` events.
    -   **keyed** (`keys=[...]`): filter the stream to specific keys.
    -   **patch-mode** (`mode=PATCH`): receive `PATCH_ITEM` deltas instead of full `ADD_ITEM` replacements.
    -   The subscribe acknowledgement acts as a **barrier** so callers can await a subscription going live.

> **Async variants:** provide `*_async` counterparts for the command operations in languages
> that have both blocking and non-blocking styles (Java, Python, Rust). TypeScript is async-only.

## API Specification

Refer to the official OpenAPI specification for exact request/response shapes, field names, and
HTTP routes:
`https://github.com/bhf/aeron-cache/blob/main/cache-http/openapi.yml`

The Aeron SBE gateway wire protocol is defined by the SBE schema at
`libraries/java/src/main/resources/sbe/gateway-schema.xml` (shared by the Java and Rust gateway
clients). Generate or hand-write codecs from this schema — do not invent a new layout.

## Data Models

Implement models to match the OpenAPI spec and the reference clients. The set includes (names
may be adapted idiomatically):

-   Responses: `CreateResponse`, `PutItemResponse`, `GetItemResponse`, `DeleteItemResponse`,
    `DeleteCacheResponse`, `ClearCacheResponse`, `GetCacheResponse`, `PatchItemResponse`,
    `CancelItemRemovalResponse`, `CounterResponse`, `GetCountersResponse`, `ErrorResponse`.
-   Items & details: `CacheItem`, `CounterItem`, `CacheDetails`.
-   Inspection: `CacheStatsResponse` / `StatEntry`, `GetTimersResponse` / `TimerInfo`.
-   Bulk: `BulkCacheOpsRequest` / `BulkCacheOpsResponse`, `CacheOperationRequest` /
    `CacheOperationResponse`, `BulkOperationType` (enum), `PutTimedItemRequest`, `PatchItemRequest`.
-   Stream events: `CacheUpdateEvent` and `CounterUpdateEvent`, each with:
    -   `eventType`: one of `ADD_ITEM`, `REMOVE_ITEM`, `PATCH_ITEM`, `CLEAR_CACHE`, `DELETE_CACHE`.
    -   `itemKey`: the key affected.
    -   `itemValue`: the value (for `ADD_ITEM`; a delta for `PATCH_ITEM`).
    -   `timestamp`: event time.

### Business Status Mapping
Responses carry an `operationStatus` field (e.g. `SUCCESS`, `CACHE_EXISTS`, `UNKNOWN_KEY`) so
callers handle business outcomes **without** throwing transport-level exceptions for HTTP 400s.
Map this into the response models rather than raising on non-200 business errors.

## Local Cache Synchronization Logic

Each embedded abstraction maintains a local map and applies stream events to it:

1.  **Initialization**: start with an empty local map and subscribe to the cache's update stream
    (optionally with `hydrate=true` to seed the current contents).
2.  **Event handler**:
    -   `ADD_ITEM`: set `local[event.itemKey] = event.itemValue`.
    -   `REMOVE_ITEM`: remove `event.itemKey`.
    -   `CLEAR_CACHE` / `DELETE_CACHE`: clear the local map.
    -   `PATCH_ITEM`:
        -   `EmbeddedAeronCache` (string): **ignore** it — a delta cannot be merged into an
            opaque string. This client also **rejects** patch-mode subscriptions.
        -   `EmbeddedObjectCache`: **deep-merge** the delta (see below).
3.  **Reads/writes**: `getLocal(key)` reads the mirror; `put`/`remove`/etc. write through to the
    server (the mirror then updates via the stream).

### Embedded Object Cache
`EmbeddedObjectCache` holds structured JSON objects and applies `PATCH_ITEM` deltas using
**RFC 7386 (JSON Merge Patch)** semantics — nested objects merge recursively, scalars/arrays
replace, and a `null` field deletes it — so patch-mode subscriptions (which stream only changed
fields) reconstruct the full object locally without losing untouched fields. Back it with the
language's natural JSON object type (Java: Jackson `ObjectNode`, with `getLocalAs` to deserialize
into a POJO; Python: `dict`; TypeScript: `Record<string, any>`; Rust: `serde_json::Value`).

## Batched Responses

Large responses (entries, stats, timers, bulk results) are streamed as one or more batches
terminated by an **end-of-batch** marker. The client must reassemble these batches into a single
result before returning. This applies across transports (bidi WebSocket and the Aeron gateway in
particular). Follow the reassembly logic in the reference clients.

## Implementation Guidelines

-   **Asynchrony**: use idiomatic async (`async/await` in Python/TypeScript/Rust,
    `CompletableFuture` in Java). Provide sync + async where appropriate.
-   **Type Safety**: strongly typed models, enums for `operationStatus` / event types /
    `BulkOperationType`.
-   **Error Handling**: distinguish **business** outcomes (via `operationStatus`, no throw) from
    **transport** failures (raise/return an error). Surface `ErrorResponse` messages.
-   **Dependencies**: prefer lightweight standard libraries for HTTP and WebSockets
    (`requests`/`aiohttp`, `fetch`, `reqwest`, Java HTTP client). For the Aeron gateway, use the
    language's official Aeron client and generate SBE codecs from the shared schema.
-   **Reconnection**: WebSocket subscriptions must handle reconnection and re-subscribe.
-   **Parity**: match the reference clients' public API, naming, and sample coverage. A complete
    language implementation ships `sync-sample`, `async-sample`, `streaming-sample`,
    `bulk-sample`, and `bidi-sample` under `samples/<language>/` (plus `aeron-sample` where the
    gateway transport is implemented).

## Example Usage Pattern

```example
client = AeronCacheClient("http://localhost:7070", "ws://localhost:7071")
client.createCache("my-cache")

# Embedded cache — writes go to the server, reads come from the local mirror.
cache = client.getCache("my-cache")
await cache.put("key", "value")
val = cache.getLocal("key")           # instant local read, kept in sync via the stream

# Counters
client.createCounterCache("counter-cache")
counters = client.getCounterCache("counter-cache")
await counters.put("requests", 10)
await counters.increment("requests", 5)   # -> 15

# Object cache + patch-mode subscription (deltas deep-merged locally)
objects = client.getObjectCache("doc-cache")
await objects.subscribe(on_event, mode=PATCH)

# Inspection
await client.getStats()
await client.getTimers()               # pending TTL timers across caches + counters

# Bidirectional WebSocket — full command surface over one connection
bidi = AeronBidiClient("ws://localhost:7071")
await bidi.putItem("bidi-cache", "k", "v")
```

See the `libraries/` and `samples/` directories for full, runnable references in all languages.
