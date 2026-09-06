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

## Counters

Counter caches hold `int64` values and add `increment`, `decrement`, and `set` operations:

```typescript
import { AeronCacheClient } from '@aeron-cache/embedded-client';

const client = new AeronCacheClient('http://localhost:7070', 'ws://localhost:7071');
await client.createCounterCache('counter-cache');
const counters = client.getCounterCache('counter-cache');

await counters.put('requests', 10);
await counters.increment('requests', 5); // -> 15
await counters.decrement('requests', 3); // -> 12
await counters.set('requests', 100);     // -> 100
console.log((await counters.get('requests')).value);
```

## Bulk Operations

```typescript
const result = await client.bulkOps({
    requestId: 'req-1',
    operations: [
        { operationType: 'INCREMENT_COUNTER', cacheId: 'counter-cache', key: 'requests', counterValue: 5 },
    ],
});
```
