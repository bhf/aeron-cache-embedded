#!/bin/bash

# Aeron Cache Embedded - Multi-Language Sample Runner
# This script runs the sync/async samples for all 4 languages.

set -e

# Setup environment
WORKSPACE_ROOT=$(pwd)
export PYTHONPATH=$PYTHONPATH:$WORKSPACE_ROOT/libraries/python

# Colors for output
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo -e "${GREEN}>>> Starting Sample Run for All Languages...${NC}"

# 1. Java
echo -e "\n${GREEN}--- Running Java Sync Sample ---${NC}"
(cd samples/java/sync-sample && ./gradlew run)

echo -e "\n${GREEN}--- Running Java Async Sample ---${NC}"
(cd samples/java/async-sample && ./gradlew run)

# 2. Python
echo -e "\n${GREEN}--- Running Python Sync Sample ---${NC}"
(cd samples/python/sync-sample && /usr/bin/python3 sync_sample.py)

echo -e "\n${GREEN}--- Running Python Async Sample ---${NC}"
(cd samples/python/async-sample && /usr/bin/python3 async_sample.py)

# 3. TypeScript
echo -e "\n${GREEN}--- Running TypeScript Async Sample ---${NC}"
(cd samples/typescript/async-sample && npm run build && node dist/index.js)

# 4. Rust
echo -e "\n${GREEN}--- Running Rust Async Sample ---${NC}"
(cd samples/rust/async-sample && cargo run -- http://localhost:7070 ws://localhost:7071)

echo -e "\n${GREEN}>>> All samples completed successfully!${NC}"
