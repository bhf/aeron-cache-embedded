# Library Generation Instructions for Aeron Cache

You are tasked with creating a polyglot client library for **Aeron Cache**. Use the following instructions to implement the library in the user's language of choice.

## Core Concepts

The Aeron Cache client should follow a tiered architecture:

1.  **Low-Level Client (`AeronCacheClient`)**:
    -   Handles direct HTTP calls to the Aeron Cache API.
    -   Manages the base HTTP URL (e.g., `http://localhost:7070`) and the WebSocket URL (e.g., `ws://localhost:7071`).
    -   Implements all endpoints defined in the OpenAPI spec.
    -   Handles WebSocket connection and reconnection logic for subscriptions.

2.  **High-Level Abstraction (`EmbeddedAeronCache`)**:
    -   Represented by a specific cache instance (obtained via `client.getCache(cacheId)`).
    -   Wraps the low-level client methods specifically for that cache ID.
    -   **Critical Feature**: Maintains a `local_cache` in-memory map/dictionary.
    -   Automatically subscribes to updates via WebSockets and keeps the `local_cache` synchronized.
    -   Provides `getLocal(key)` for instant lookups from the embedded cache.

## API Specification

Refer to the official OpenAPI specification:
`https://github.com/bhf/aeron-cache/blob/main/cache-http/openapi.yml`

### Key Endpoints

- `POST /api/v1/cache`: Create a new cache.
- `GET /api/v1/cache/{cacheId}`: Retrieve all items in a cache.
- `POST /api/v1/cache/{cacheId}`: Put an item into a cache.
- `DELETE /api/v1/cache/{cacheId}`: Delete a cache.
- `PATCH /api/v1/cache/{cacheId}`: Clear all items in a cache.
- `GET /api/v1/cache/{cacheId}/{key}`: Get a specific item.
- `DELETE /api/v1/cache/{cacheId}/{key}`: Delete a specific item.
- `GET /api/v1/caches`: List all available caches.

### WebSocket Subscriptions

The client must support subscribing to cache updates at:
`${WS_URL}/api/ws/v1/cache/${cacheId}`

The server sends messages as `CacheUpdateEvent` objects.

## Data Models

Implement the following models (refer to OpenAPI for exact fields):

- `CreateRequest` / `CreateResponse`
- `PutItemRequest` / `PutItemResponse`
- `GetItemResponse`
- `DeleteItemResponse`
- `DeleteCacheResponse`
- `ClearCacheResponse`
- `CacheItem`
- `CacheDetails`
- `CacheUpdateEvent`:
    - `eventType`: `ADD_ITEM`, `REMOVE_ITEM`, `CLEAR_CACHE`, `DELETE_CACHE`.
    - `itemKey`: The key affected.
    - `itemValue`: The value (if applicable).
    - `timestamp`: Event time.

## Local Cache Synchronization Logic

Inside `EmbeddedAeronCache`:

1.  **Initialization**:
    -   Initialize an empty map `local_cache`.
    -   Immediately start a background task to `subscribe` to the cache.
2.  **WebSocket Message Handler**:
    -   `ADD_ITEM`: Add/update `local_cache[event.itemKey] = event.itemValue`.
    -   `REMOVE_ITEM`: Remove `event.itemKey` from `local_cache`.
    -   `CLEAR_CACHE` / `DELETE_CACHE`: Clear the `local_cache`.
3.  **Methods**:
    -   `get(key)`: Fetch from remote (HTTP).
    -   `getLocal(key)`: Return from `local_cache`.
    -   `put(key, value)`: Send to remote (HTTP).
    -   `remove(key)`: Send to remote (HTTP).

## Implementation Guidelines

-   **Asynchrony**: Use the language's idiomatic async patterns (e.g., `async/await` in Python/Typescript/Rust, `CompletableFuture` in Java).
-   **Type Safety**: Use strongly typed models and interfaces.
-   **Error Handling**: Raise appropriate exceptions or return error types with descriptive messages from `ErrorResponse`.
-   **Dependencies**: Use lightweight, standard libraries for HTTP (e.g., `requests`/`aiohttp`, `fetch`, `reqwest`) and WebSockets.
-   **Samples**: Refer to existing implementations in `aeron-cache-embedded/libraries` for reference.

## Example Usage Pattern

```example
client = AeronCacheClient("http://localhost:7070", "ws://localhost:7071")
cache = client.getCache("my-cache")

# Subscribe to a callback if the user wants custom logic
cache.subscribe(lambda event: print(f"Received: {event}"))

# Remote write
await cache.put("key", "value")

# Local read (updated via WS)
val = cache.getLocal("key")
```
