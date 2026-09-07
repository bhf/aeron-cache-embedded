# Aeron Gateway Sample (Rust)

Demonstrates the **Aeron transport** for Aeron Cache using
[`AeronGatewayClient`](../../../libraries/rust/src/gateway.rs) — the low-latency, bidirectional
alternative to the HTTP+WS client. It covers cache CRUD, a streaming subscription, and counter
operations, all over a single Aeron connection.

## Prerequisites

The backend must have the Aeron gateway enabled:

```
AERON_TRANSPORT_GATEWAY_ENABLED=true
```

The gateway binds the request endpoint on port `7075` and the response control endpoint on port `7076`
by default. Building requires a C compiler, `cmake`, and `libclang` (the Aeron transport wraps the
Aeron C client via `rusteron-client`).

## Running

The sample launches its own embedded media driver, so no separate Aeron media driver is needed.

```bash
cargo run
```

Point it at a non-local gateway with:

```bash
cargo run -- <host>
```
