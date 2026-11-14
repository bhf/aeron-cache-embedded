# Aeron Cache Embedded Samples

This folder contains example applications demonstrating how to use the Aeron Cache Embedded Clients in Java, TypeScript, Python, and Rust.

The samples for each language are split into distinct categories:
1.  **Sync Sample**: Demonstrates sequential, synchronous-style operations.
2.  **Async Sample**: Demonstrates non-blocking, asynchronous operations.
3.  **Streaming Sample**: Demonstrates how to listen for real-time updates from the server.
4.  **Bulk Sample**: Demonstrates executing multiple operations in a single request.
5.  **Patch Stream Sample**: Demonstrates server-side synthetic PATCH streaming — a patch-mode subscription
    receives an `ADD_ITEM` baseline then `PATCH_ITEM` deltas (driven by ordinary puts), reconstructed into
    full objects by the embedded object cache.
6.  **Bidi Sample**: Demonstrates the bidirectional WebSocket transport (`AeronBidiClient`) — the full cache +
    counter command surface plus live subscribe/unsubscribe multiplexed over one persistent WebSocket.
7.  **Aeron Sample** (Java and Rust only): Demonstrates the Aeron gateway transport (`AeronGatewayClient`) —
    a low-latency, bidirectional alternative to HTTP+WS. Requires the backend's Aeron gateway to be enabled.

## Structure

```
samples/
├── java/                    # Gradle projects
│   ├── sync-sample/
│   ├── async-sample/
│   ├── streaming-sample/
│   ├── bulk-sample/
│   ├── patch-stream-sample/
│   ├── bidi-sample/
│   └── aeron-sample/
├── typescript/              # NPM projects (no sync or aeron sample)
│   ├── async-sample/
│   ├── streaming-sample/
│   ├── bulk-sample/
│   ├── patch-stream-sample/
│   └── bidi-sample/
├── python/                  # Poetry projects (no aeron sample)
│   ├── sync-sample/
│   ├── async-sample/
│   ├── streaming-sample/
│   ├── bulk-sample/
│   ├── patch-stream-sample/
│   └── bidi-sample/
└── rust/                    # Cargo projects
    ├── sync-sample/
    ├── async-sample/
    ├── streaming-sample/
    ├── bulk-sample/
    ├── patch-stream-sample/
    ├── bidi-sample/
    └── aeron-sample/
```

Not every sample has its own README; check the sample folder for run instructions.

## Running the Samples

Each sample is a self-contained project. Please navigate to the specific subdirectory and refer to its specific `README.md` (where present) for run instructions.

### Quick Reference

- **Java**: Uses Gradle. Run `./gradlew run --args="<url>"` inside the sample folder.
- **TypeScript**: Uses NPM. Run `npm install && npm run build && npm start -- <url>` inside the sample folder.
- **Python**: Uses Poetry. Run `poetry install && poetry run python <script> <url>` inside the sample folder.
- **Rust**: Uses Cargo. Run `cargo run -- <url>` inside the sample folder.

The repo root also has `run-all-samples.sh` (sync/async samples) and `run-streaming-samples.sh`
(streaming and patch-stream samples in parallel), both run from the repository root.

## Continuous Integration

The [Samples CI](../.github/workflows/samples-ci.yml) workflow builds every sample against the in-repo
client libraries on each push/PR touching `samples/` or `libraries/`. It is build-only — samples are not
executed and no backend is started. Java samples are built with the local library substituted for the
pinned published version; Python samples are byte-compiled as a syntax check.
