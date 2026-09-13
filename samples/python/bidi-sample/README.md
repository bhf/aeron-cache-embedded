# Python Bidi Sample

This sample demonstrates the **bidirectional WebSocket transport** (`AeronBidiClient`),
which multiplexes the full cache + counter command surface plus live subscribe/unsubscribe
over a single persistent WebSocket connection.

It covers: create cache, put/get, patch (merge), a counter demo
(create/put/increment/get), bulk read + stats, a live streamed subscription, and teardown.

## Prerequisites

- Python 3.10+
- A running backend (HTTP on 7070, WebSocket on 7071)
- Poetry (optional; only for the `poetry` workflow below)

## Running the Sample

### With Poetry

1. Install dependencies (installs the `aeron_cache` client as an editable dependency):
   ```bash
   poetry install
   ```

2. Run the sample:
   ```bash
   poetry run python bidi_sample.py ws://localhost:7071
   ```

### With PYTHONPATH

The client package lives at `libraries/python/aeron_cache`. Point `PYTHONPATH` at
`libraries/python` to make `aeron_cache` importable (the `websockets` dependency must be
installed — the repo venv at `libraries/python/.venv` already has it):

```bash
PYTHONPATH=../../../libraries/python python bidi_sample.py ws://localhost:7071
```

The ws URL defaults to `ws://localhost:7071` if omitted.
