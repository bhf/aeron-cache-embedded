# Aeron Gateway Sample (Rust)

Demonstrates the **Aeron transport** for Aeron Cache using
[`AeronGatewayClient`](../../../libraries/rust/src/gateway.rs) — the low-latency, bidirectional
alternative to the HTTP+WS client. It covers cache CRUD, a streaming subscription, and counter
operations, all over a single Aeron connection.

## Prerequisites

The backend must have the Aeron gateway enabled:

```
AERON_GATEWAY_ENABLED=true
```

The gateway binds the request endpoint on port `7075` and the response control endpoint on port `7076`
by default. Building requires a C compiler, `cmake`, and `libclang` (the Aeron transport wraps the
Aeron C client via `rusteron-client`).

## Running

By default the sample connects over **UDP**, launching its own embedded media driver, so no separate
Aeron media driver is needed.

```bash
cargo run
```

Point it at a non-local gateway with:

```bash
cargo run -- <host>
```

### IPC transport media

The gateway also supports **IPC** for co-located clients — lower latency, no network stack, but the
client and gateway server must share the same media driver (same host, same `aeron.dir`). Start the
backend with the shared directory and `GATEWAY_TRANSPORT_MEDIA=ipc`:

```bash
AERON_DIR=/tmp/aeron-cache-shared GATEWAY_TRANSPORT_MEDIA=ipc aeron-cache
```

Then run the sample against the same directory, over IPC:

```bash
AERON_GATEWAY_MEDIA=ipc AERON_DIR=/tmp/aeron-cache-shared cargo run
```

For IPC the sample does **not** launch an embedded driver — `AERON_DIR` must point at the driver the
gateway server is using, and the host CLI argument is ignored (IPC has no network endpoints).
