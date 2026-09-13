# Bidi WebSocket Sample (Java)

Demonstrates the **bidirectional WebSocket transport** for Aeron Cache using
[`AeronBidiClient`](../../../libraries/java/src/main/java/com/bhf/aeroncache/client/bidi/AeronBidiClient.java) —
the JSON/WebSocket analogue of the Aeron gateway client. A single persistent WebSocket connection
(`/api/ws/v1/bidi`) carries the full cache + counter command surface plus dynamic
subscribe/unsubscribe, multiplexed by a client-minted correlation id. It covers:

- cache CRUD (`createCache`, `putItem`, `getItem`),
- JSON merge patching (`patchItem`),
- counters (`createCounterCache`, `putCounter`, `incrementCounter`, `getCounter`),
- bulk reads and stats (`getCacheItems`, `getStats`),
- a live streaming subscription over the same connection.

## Prerequisites

A running backend with the WebSocket transport reachable (HTTP on `7070`, WebSocket on `7071` by
default). No separate Aeron media driver is required — the bidi transport is pure JSON over
WebSocket.

## Running

```bash
./gradlew run
```

Point it at a non-default WebSocket URL by passing it as the first argument:

```bash
./gradlew run --args="ws://localhost:7071"
```

The URL defaults to `ws://localhost:7071` when no argument is given.
