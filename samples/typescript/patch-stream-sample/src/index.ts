import WebSocket from 'ws';

// Polyfill global WebSocket for Node.js environment
// @ts-ignore
global.WebSocket = WebSocket;

import { AeronCacheClient } from '@aeron-cache/embedded-client';

/**
 * Demonstrates server-side synthetic PATCH streaming.
 *
 * Subscribes in `patch` mode and then writes evolving *full* JSON values with ordinary `putItem` calls
 * (never `patchItem`). As long as a patch-mode subscriber is listening, the cache service turns those puts
 * into patch traffic: the first put of a key arrives as a full ADD_ITEM baseline, each later overwrite as a
 * PATCH_ITEM delta of only the changed fields, an identical (no-op) put is suppressed, and a non-JSON value
 * falls back to a full ADD_ITEM. The EmbeddedObjectCache deep-merges each delta (RFC 7386), so get
 * always holds the full, reconstructed object.
 */
async function main() {
    const baseUrl = process.argv[2] || 'http://localhost:7070';
    const wsUrl = process.argv[3] || 'http://localhost:7071';

    console.log(`Starting Patch Stream Sample against ${baseUrl}, ws=${wsUrl}`);

    const client = new AeronCacheClient(baseUrl, wsUrl);
    const cacheId = 'patch-stream-sample-cache';
    try {
        await client.createCache(cacheId);
    } catch (e) {
        // Probably already exists.
    }

    // Start from a clean slate so the first put is a fresh insert (an ADD_ITEM baseline) rather than an
    // overwrite of a value left over from a previous run (which would arrive as a PATCH_ITEM delta).
    try {
        await client.deleteItem(cacheId, 'doc');
    } catch (e) {
        // Key may not exist.
    }

    const oc = client.getObjectCache(cacheId);
    const onMessage = (event: any) => {
        if (event.itemKey !== 'doc') return;
        console.info(
            `[TypeScript] <- ${event.eventType} delta=${event.itemValue}  reconstructed=${JSON.stringify(oc.get('doc'))}`
        );
    };
    const onError = (err: any) => console.error('[TypeScript] subscription error', err);

    // Subscribe in patch mode BEFORE the key exists, so the first put arrives as a full ADD_ITEM baseline
    // and every later put as a PATCH_ITEM delta the object cache deep-merges locally.
    oc.subscribe(onMessage, onError, undefined, false, undefined, 'patch');

    const states = [
        '{"user":{"name":"ada","age":36},"status":"active"}',
        '{"user":{"name":"ada","age":37},"status":"active"}', // delta: user.age -> 37
        '{"user":{"name":"ada","age":37},"status":"away"}',   // delta: status -> away
        '{"user":{"name":"ada","age":37},"status":"away"}',   // identical -> suppressed
        '{"user":{"name":"ada","age":37}}'                      // delta: status -> null (removed)
    ];

    await new Promise((r) => setTimeout(r, 800)); // let the subscription register before the first put

    let i = 0;
    const tick = async () => {
        const state = states[i % states.length];
        console.log(`[TypeScript] -> put doc = ${state}`);
        try {
            await client.putItem(cacheId, 'doc', state);
        } catch (e) {
            console.error(e);
        }
        i++;
    };

    await tick();
    setInterval(tick, 2500);

    // Keep the process alive to keep receiving updates.
    await new Promise(() => {});
}

main();
