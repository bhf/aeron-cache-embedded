# Aeron Cache Embedded Clients

[![Java CI](https://github.com/bhf/aeron-cache-embedded/actions/workflows/java-ci.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/java-ci.yml)
[![TypeScript CI](https://github.com/bhf/aeron-cache-embedded/actions/workflows/typescript-ci.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/typescript-ci.yml)
[![Python CI](https://github.com/bhf/aeron-cache-embedded/actions/workflows/python-ci.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/python-ci.yml)
[![Rust CI](https://github.com/bhf/aeron-cache-embedded/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/bhf/aeron-cache-embedded/actions/workflows/rust-ci.yml)

Embedded cache client SDKs for Aeron Cache in multiple languages: Java, TypeScript, Python, and Rust with minimal external dependencies.

The goal of these libraries is to provide an "Embedded Cache" mode across languages. In this mode, the client maintains a local copy of the cache data which is kept in sync with the server via WebSocket updates, allowing for fast local reads.

## Functionality and Samples By Language

| Language   | Sync | Async | Streaming |
|:-----------|:----|:------|:-------|
| Java       | ✅   | ✅     | ✅      |
| Typescript | NA  | ✅     | ✅      |
| Python     | ✅   | ✅     | ✅      |
| Rust       | ✅     | ✅     | ✅       |

## Example Usage

### Java

```typescript
var baseUrl = "http://localhost:7070";
var wsUrl = "http://localhost:7071";
AeronCacheClient client = new AeronCacheClient(baseUrl, wsUrl);
try {
    client.createCache("async-sample-cache");
    System.out.println("Created cache 'async-sample-cache'");
} catch (Exception e) {
    e.printStackTrace();
}
EmbeddedAeronCache cache = new EmbeddedAeronCache(client, "async-sample-cache");
cache.put("stay", "tuned");
```

### Typescript

```typescript
const baseUrl = "http://localhost:7070";
const wsUrl = "http://localhost:7071";
const cacheClient = new AeronCacheClient(baseUrl, wsUrl);

try {
    await cacheClient.createCache('async-sample-cache');
} catch (e) {
    console.error(e);
}

const cache = new EmbeddedAeronCache(cacheClient, 'async-sample-cache');
await cache.put("stay", "tuned");
```

See the `samples/` directory for full code examples in all languages.

## Running Samples

You can run all samples using the provided helper scripts:

### Standard Samples (Sync/Async)
This script runs the basic sync and async samples for all languages sequentially.
```bash
./run-all-samples.sh
```

### Streaming Samples (Embedded Cache)
This script runs the streaming (Embedded Cache) samples for all languages in **parallel**. It captures the output from all languages into a single terminal window.
```bash
./run-streaming-samples.sh
```
*Note: Press Ctrl+C to stop all parallel streaming processes.*

## Features

-   **Business Status Mapping**: Responses include an `operationStatus` field (e.g., `SUCCESS`, `CACHE_EXISTS`, `UNKNOWN_KEY`) to handle business logic without throwing transport-level exceptions for HTTP 400 errors.
-   **CRUD Operations**: Full support for Create, Get, Put, Delete items and caches.
-   **Sync & Async**: APIs available in both synchronous (blocking) and asynchronous (non-blocking) styles where appropriate.
-   **Embedded Aeron Cache**: A specialized `EmbeddedAeronCache` object that maintains a local shadowed copy of the cache data. It subscribes to the cache's WebSocket stream and applies updates (`ADD_ITEM`, `REMOVE_ITEM`, `DELETE_CACHE`, `CLEAR_CACHE`) to the local map automatically.


## Project Structure

-   `libraries/`: Core client libraries.
    -   `java/`: Java client library.
    -   `typescript/`: TypeScript/JavaScript client library.
    -   `python/`: Python client library.
    -   `rust/`: Rust client library.
-   `samples/`: Example applications using each of the clients.

