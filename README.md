# Aeron Cache Embedded Clients

Embedded cache client SDKs for Aeron Cache in multiple languages: Java, TypeScript, Python, and Rust with minimal external dependencies.

The goal of these libraries is to provide an "Embedded Cache" mode across languages. In this mode, the client maintains a local copy of the cache data which is kept in sync with the server via WebSocket updates, allowing for fast local reads.

## Project Structure

-   `libraries/`: Core client libraries.
    -   `java/`: Java client library.
    -   `typescript/`: TypeScript/JavaScript client library.
    -   `python/`: Python client library.
    -   `rust/`: Rust client library.
-   `samples/`: Example applications using each of the clients.

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

## Usage

### Java

```java
// See samples/java/ for full examples
EmbeddedAeronCache cache = new EmbeddedAeronCache("http://localhost:8080");
// ... creates a local map replicating the remote cache
```
String val = cache.getLocal("key");

// Remote writes (propagated back via WebSocket)
cache.put("key", "value");
```

### Python

```python
client = AeronCacheClient("http://localhost:7070", "ws://localhost:7071")
cache = client.get_cache("my-cache")

# Async subscription
await cache.subscribe(my_callback)
```

See the `samples/` directory for full code examples in all languages.
