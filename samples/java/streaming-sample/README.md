# Java Streaming Sample

This sample demonstrates how to listen for real-time updates from the cache using WebSocket integration (Streaming).

## Prerequisites

- Java 11 or higher
- Gradle

## Running the Sample

1. Navigate to this directory.
2. Run the application using Gradle:

```bash
gradle run --args="http://localhost:8080"
```

The application will listen for changes to the key `shared-key`. You can modify this key via another client (e.g., using `curl` or another sample) to see the updates in real-time.
