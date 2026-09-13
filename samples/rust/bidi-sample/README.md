# BIDI WebSocket Sample (Rust)

Demonstrates the **bidirectional WebSocket transport** for Aeron Cache using
[`AeronBidiClient`](../../../libraries/rust/src/bidi.rs) — a single `/api/ws/v1/bidi`
WebSocket connection that carries the full cache + counter command surface plus dynamic
subscribe/unsubscribe, multiplexed by a client-minted correlation id. It covers cache CRUD,
JSON patch/merge, a streaming subscription, counters, and stats over one connection.

## Prerequisites

The backend must be running with the WebSocket endpoint available (WS `7071` by default, HTTP
`7070`). Building requires a C compiler, `cmake`, and `libclang` (the shared client crate pulls in
the Aeron transport, which wraps the Aeron C client via `rusteron-client`).

## Running

```bash
cargo run
```

Point it at a non-local endpoint with:

```bash
cargo run -- ws://<host>:7071
```
