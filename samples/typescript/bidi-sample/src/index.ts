import { AeronBidiClient, CacheUpdateEvent } from '@aeron-cache/embedded-client';

async function main() {
    const wsUrl = process.argv[2] || 'ws://localhost:7071';

    console.log(`Starting Bidi Sample against ${wsUrl}`);

    const client = new AeronBidiClient(wsUrl);

    try {
        await client.connect();
        console.log('Connected to bidirectional WebSocket transport');
    } catch (err) {
        console.error(`Failed to connect to ${wsUrl}: ${(err as Error).message}`);
        await client.close();
        process.exit(1);
    }

    const cacheId = 'bidi-sample-cache';
    const counterCacheId = 'bidi-sample-counter-cache';

    try {
        // --- Cache lifecycle + basic get/put ---
        const created = await client.createCache(cacheId);
        console.log(`Created cache '${created.cacheId}' (status: ${created.operationStatus})`);

        console.log("Putting key 'greeting' -> 'hello from bidi'");
        const putResp = await client.putItem(cacheId, 'greeting', 'hello from bidi');
        console.log(`Put status: ${putResp.operationStatus}`);

        const getResp = await client.getItem(cacheId, 'greeting');
        console.log(`Read key 'greeting': ${getResp.value || 'not found'}`);

        // --- Patch (deep-merge) ---
        console.log("Putting 'doc' -> {\"a\":1}, then patching with {\"b\":2}");
        await client.putItem(cacheId, 'doc', '{"a":1}');
        const patchResp = await client.patchItem(cacheId, 'doc', '{"b":2}');
        console.log(`Patch status: ${patchResp.operationStatus}`);
        console.log(`Doc after patch: ${(await client.getItem(cacheId, 'doc')).value}`);

        // --- Counters ---
        const counterCreated = await client.createCounterCache(counterCacheId);
        console.log(`Created counter cache '${counterCreated.cacheId}' (status: ${counterCreated.operationStatus})`);

        console.log("Putting counter 'requests' -> 10");
        await client.putCounter(counterCacheId, 'requests', 10);
        const incremented = await client.incrementCounter(counterCacheId, 'requests', 5);
        console.log(`Incremented 'requests' by 5 -> ${incremented.value}`);
        const counterGet = await client.getCounter(counterCacheId, 'requests');
        console.log(`Read counter 'requests': ${counterGet.value}`);

        // --- Inspection ---
        console.log(`Listing all items in '${cacheId}':`);
        for (const item of (await client.getCacheItems(cacheId)).items) {
            console.log(`  - ${item.key} = ${item.value}`);
        }

        console.log('Cache stats (per-cache):');
        for (const stat of await client.getStats()) {
            console.log(`  - ${stat.cacheId}: size=${stat.size} added=${stat.addedCount} removed=${stat.removedCount} cleared=${stat.clearedCount}`);
        }

        // --- Live subscription over the same connection ---
        console.log(`Subscribing to '${cacheId}' for live updates`);
        let resolveEvent: (ev: CacheUpdateEvent) => void;
        const eventPromise = new Promise<CacheUpdateEvent>((resolve) => {
            resolveEvent = resolve;
        });
        const sub = await client.subscribe(cacheId, (ev) => resolveEvent(ev));

        // Trigger an update on the subscribed cache; the listener resolves the promise.
        console.log("Putting key 'live-key' -> 'streamed-value' to trigger an update");
        await client.putItem(cacheId, 'live-key', 'streamed-value');

        const event = await eventPromise;
        await sub.close();
        console.log(`Received CacheUpdateEvent: cacheId=${event.cacheId} eventType=${event.eventType} itemKey=${event.itemKey} itemValue=${event.itemValue}`);
        console.log('Closed subscription');

        // --- Cleanup ---
        const deleted = await client.deleteCache(cacheId);
        console.log(`Deleted cache '${deleted.cacheId}' (status: ${deleted.operationStatus})`);
        await client.deleteCounterCache(counterCacheId);
        console.log(`Deleted counter cache '${counterCacheId}'`);
    } catch (err) {
        console.error(`Error during bidi sample: ${(err as Error).message}`);
    } finally {
        await client.close();
        console.log('Client closed');
    }
}

main();
