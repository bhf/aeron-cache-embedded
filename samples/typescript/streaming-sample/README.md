# TypeScript Streaming Sample

This sample demonstrates long-lived WebSocket subscriptions to cache updates (Streaming).

## Prerequisites

- Node.js (v14+)
- NPM

## Running the Sample

1. Install dependencies:
   ```bash
   npm install
   ```

2. Build the TypeScript code:
   ```bash
   npm run build
   ```

3. Run the sample:
   ```bash
   npm start -- http://localhost:8080
   ```

The script will keep running and print updates whenever `shared-key` changes on the server.
