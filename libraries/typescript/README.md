# Aeron Cache TypeScript Client

This is the TypeScript/JavaScript client library for Aeron Cache.

## Installation

```bash
npm install @aeron-cache/embedded-client
```

## Usage

```typescript
import { AeronCacheClient } from '@aeron-cache/embedded-client';

async function main() {
    const client = new AeronCacheClient('http://localhost:7070', 'ws://localhost:7071');
    const cache = client.getCache('my-cache');

    // Subscribe to updates
    cache.subscribe((data) => console.log('Update:', data));

    // Remote writes
    await cache.set('key', 'value');

    // Local reads (from embedded cache)
    console.log(cache.getLocal('key'));
}

main();
```
