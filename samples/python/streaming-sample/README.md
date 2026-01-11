# Python Streaming Sample

This sample demonstrates listening for real-time updates via WebSocket (Streaming).

## Prerequisites

- Python 3.7+
- Poetry

## Running the Sample

1. Install dependencies:
   ```bash
   poetry install
   ```

2. Run the sample:
   ```bash
   poetry run python streaming_sample.py http://localhost:8080
   ```

The script will listen for changes to `shared-key` indefinitely.
