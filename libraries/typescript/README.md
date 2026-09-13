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

Counter caches hold `int64` values and add `increment`, `decrement`, `set`, and timed-put operations:

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

Counter operations are also available via `bulkOps` using the counter `BulkOperationType` values and the `counterValue` field on `CacheOperationRequest`.

## Inspection & management operations

Beyond the basic CRUD surface, the client exposes operations for inspecting and managing caches and counters:

```typescript
const client = new AeronCacheClient('http://localhost:7070', 'ws://localhost:7071');

// --- Caches ---
// Deep-merge a JSON document into an existing item
await client.patchItem('my-cache', 'doc', '{"b":2}');

// Cancel a pending timed removal so the item is kept
await client.cancelItemRemoval('my-cache', 'key');

// List every cache with its item count
const caches = await client.getCaches();       // CacheDetails[]

// Server-wide cache statistics
const stats = await client.getStats();          // CacheStatsResponse

// --- Counters ---
// List every counter in a cache
const counters = await client.getCounterItems('counter-cache');  // GetCountersResponse

// Clear all counters in a cache
await client.clearCounterCache('counter-cache');

// Cancel a pending timed removal of a counter
await client.cancelCounterItemRemoval('counter-cache', 'hits');

// List every counter cache with its item count
const counterCaches = await client.getCounterCaches();  // CacheDetails[]

// Server-wide counter statistics
const counterStats = await client.getCounterStats();    // CacheStatsResponse
```

## Bidirectional WebSocket transport

`AeronBidiClient` runs the entire cache + counter command surface, plus dynamic
subscribe/unsubscribe, over a single persistent WebSocket to `/api/ws/v1/bidi`.
It is the JSON/WebSocket analogue of the Aeron gateway transport and an alternative
to the HTTP-based `AeronCacheClient`. Every method returns a `Promise`; commands are
multiplexed by a client-minted `correlationId` that the server echoes back.

```typescript
import { AeronBidiClient } from '@aeron-cache/embedded-client';

async function main() {
    const client = new AeronBidiClient('ws://localhost:7071');
    await client.connect();

    // Cache commands (mirror the HTTP client)
    await client.createCache('my-cache');
    await client.putItem('my-cache', 'key', 'value');
    console.log((await client.getItem('my-cache', 'key')).value);   // 'value'
    console.log(await client.getCacheItems('my-cache'));            // GetCacheResponse
    console.log(await client.getStats());                          // StatEntry[] (per-cache)

    // Counter commands
    await client.createCounterCache('counters');
    await client.putCounter('counters', 'hits', 10);
    console.log((await client.incrementCounter('counters', 'hits', 5)).value); // 15

    // Live subscriptions — updates route by cacheId. Values are strings for caches
    // (CacheUpdateEvent) and numbers for counters (CounterUpdateEvent).
    const sub = await client.subscribe('my-cache', (ev) => {
        console.log(ev.eventType, ev.itemKey, ev.itemValue);
    });
    // ...later:
    await sub.close();   // alias: sub.unsubscribe()

    await client.close();
}

main();
```

Optional subscription selectors: `subscribe(cacheId, onEvent, { sendSnapshot, key, mode })`
(`mode` is `'full'` or `'patch'`, cache-only) and `subscribeCounter(cacheId, onEvent, { sendSnapshot, key })`.
The client uses the runtime global `WebSocket` (browsers and Node.js >= 22).
