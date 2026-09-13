# TypeScript Bidi Sample

This sample demonstrates the bidirectional WebSocket transport (`AeronBidiClient`), where a
single persistent WebSocket connection carries the full cache + counter command surface plus
live subscriptions, multiplexed by correlation id.

## Prerequisites

- Node.js (v22+)
- NPM
- A running backend (HTTP 7070, WS 7071)

## Running the Sample

1. Install dependencies:
   ```bash
   npm install
   ```

2. Build the TypeScript code:
   ```bash
   npm run build
   ```

3. Run the sample (defaults to `ws://localhost:7071`):
   ```bash
   npm start
   ```

   Or pass an explicit WebSocket URL:
   ```bash
   node dist/index.js ws://localhost:7071
   ```
