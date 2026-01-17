# Rust Streaming Sample

This sample demonstrates the live update capabilities of the embedded cache client (Streaming).

## Prerequisites

- Rust (Cargo)

## Running the Sample

1. Run the sample using Cargo:

```bash
cargo run
```

The application will obtain a lock on the `shared-key` and print changes as they arrive via WebSocket replication.
