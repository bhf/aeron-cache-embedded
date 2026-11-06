#!/bin/bash

# Aeron Cache Embedded - Parallel Streaming Sample Runner
# This script runs all streaming samples and patch-stream samples (synthetic PATCH streaming) in
# parallel across the four languages and stops them together.

# Setup environment
WORKSPACE_ROOT=$(pwd)
export PYTHONPATH=$PYTHONPATH:$WORKSPACE_ROOT/libraries/python

# Colors for output
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}>>> Starting Parallel Streaming Sample Run...${NC}"

# PIDs to keep track of processes
PIDS=()

# Cleanup function on EXIT
cleanup() {
    echo -e "\n${BLUE}>>> Stopping all samples...${NC}"
    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null
        fi
    done
    wait 2>/dev/null
    echo -e "${BLUE}>>> All samples stopped.${NC}"
    exit 0
}

trap cleanup EXIT INT TERM

# Prepare
(cd samples/typescript/streaming-sample && npm install --quiet && npm run build --quiet)
(cd samples/typescript/patch-stream-sample && npm install --quiet && npm run build --quiet)

# 1. Java
echo -e "${BLUE}Starting Java Streaming Sample...${NC}"
(cd samples/java/streaming-sample && ./gradlew run --quiet) &
PIDS+=($!)

# 2. Python
echo -e "${BLUE}Starting Python Streaming Sample...${NC}"
(cd samples/python/streaming-sample && /usr/bin/python3 streaming_sample.py http://localhost:7070 ws://localhost:7071) &
PIDS+=($!)

# 3. TypeScript
echo -e "${BLUE}Starting TypeScript Streaming Sample...${NC}"
(cd samples/typescript/streaming-sample && node dist/index.js http://localhost:7070 ws://localhost:7071) &
PIDS+=($!)

# 4. Rust
echo -e "${BLUE}Starting Rust Streaming Sample...${NC}"
(cd samples/rust/streaming-sample && cargo run --quiet -- http://localhost:7070 ws://localhost:7071) &
PIDS+=($!)

# --- Patch-stream samples (synthetic PATCH streaming: baseline + deltas reconstructed by the object cache) ---

# 5. Java
echo -e "${BLUE}Starting Java Patch Stream Sample...${NC}"
(cd samples/java/patch-stream-sample && ./gradlew run --quiet --args="http://localhost:7070 ws://localhost:7071") &
PIDS+=($!)

# 6. Python
echo -e "${BLUE}Starting Python Patch Stream Sample...${NC}"
(cd samples/python/patch-stream-sample && /usr/bin/python3 patch_stream_sample.py http://localhost:7070 ws://localhost:7071) &
PIDS+=($!)

# 7. TypeScript
echo -e "${BLUE}Starting TypeScript Patch Stream Sample...${NC}"
(cd samples/typescript/patch-stream-sample && node dist/index.js http://localhost:7070 ws://localhost:7071) &
PIDS+=($!)

# 8. Rust
echo -e "${BLUE}Starting Rust Patch Stream Sample...${NC}"
(cd samples/rust/patch-stream-sample && cargo run --quiet -- http://localhost:7070 ws://localhost:7071) &
PIDS+=($!)

echo -e "${BLUE}>>> All samples are running. Press Ctrl+C to stop.${NC}"

# Wait for children
wait
