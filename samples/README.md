# Aeron Cache Embedded Samples

This folder contains example applications demonstrating how to use the Aeron Cache Embedded Clients in Java, TypeScript, Python, and Rust.

The samples for each language are split into three distinct categories:
1.  **Sync Sample**: Demonstrates sequential, synchronous-style operations.
2.  **Async Sample**: Demonstrates non-blocking, asynchronous operations.
3.  **Streaming Sample**: Demonstrates how to listen for real-time updates from the server.

## Structure

```
samples/
├── java/
│   ├── sync-sample/      # Gradle project
│   ├── async-sample/     # Gradle project
│   └── streaming-sample/ # Gradle project
├── typescript/
│   ├── async-sample/     # NPM project
│   └── streaming-sample/ # NPM project
├── python/
│   ├── sync-sample/      # Poetry project
│   ├── async-sample/     # Poetry project
│   └── streaming-sample/ # Poetry project
└── rust/
    ├── sync-sample/      # Cargo project
    ├── async-sample/     # Cargo project
    └── streaming-sample/ # Cargo project
```

## Running the Samples

Each sample is a self-contained project. Please navigate to the specific subdirectory and refer to its specific `README.md` for run instructions.

### Quick Reference

- **Java**: Uses Gradle. Run `gradle run --args="<url>"` inside the sample folder.
- **TypeScript**: Uses NPM. Run `npm install && npm start -- <url>` inside the sample folder.
- **Python**: Uses Poetry. Run `poetry install && poetry run python <script> <url>` inside the sample folder.
- **Rust**: Uses Cargo. Run `cargo run -- <url>` inside the sample folder.

