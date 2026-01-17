# Aeron Cache Embedded Clients

Embedded cache client SDKs for Aeron Cache in multiple languages: Java, TypeScript, Python, and Rust with minimal external dependencies.

The goal of these libraries is to provide an "Embedded Cache" mode across languages. In this mode, the client maintains a local copy of the cache data which is kept in sync with the server via WebSocket updates, allowing for fast local reads.

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

## Project Structure

-   `libraries/`: Core client libraries.
    -   `java/`: Java client library.
    -   `typescript/`: TypeScript/JavaScript client library.
    -   `python/`: Python client library.
    -   `rust/`: Rust client library.
-   `samples/`: Example applications using each of the clients.

## Tested and Working Samples

| Language   | Sync | Async | Streaming |
|:-----------|:-----|:------|:---------|
| Java       | ✅    | ✅     | ✅        |
| Typescript | ❌    | ✅     | ✅        |
| Python     | ✅     | ✅     | ✅       |
| Rust       |      |       |          |


## Features

-   **CRUD Operations**: Full support for Create, Get, Put, Delete items and caches.
-   **Sync & Async**: APIs available in both synchronous (blocking) and asynchronous (non-blocking) styles where appropriate.
-   **Embedded Aeron Cache**: A specialized `EmbeddedAeronCache` object that wraps a local Map/HashMap. It subscribes to the cache's WebSocket stream and applies updates (Add, Remove, Clear) to the local map automatically.

## Usage & Building

Each client library is packaged using standard tools for its ecosystem.

### Java
The Java client is a Gradle project.
- **Build**: `gradle build` in the `libraries/java/` directory.

### TypeScript
The TypeScript client is an NPM package.
- **Build**: `npm install && npm run build` in the `libraries/typescript/` directory.

### Python
The Python client uses Poetry.
- **Build**: `poetry install` in the `libraries/python/` directory.

### Rust
The Rust client uses Cargo.
- **Build**: `cargo build` in the `libraries/rust/` directory.

## Running Samples

The `samples/` directory contains isolated examples for each language, broken down by usage pattern (Sync, Async, WebSocket).
Please refer to the [Samples README](samples/README.md) for detailed instructions on how to run them.


