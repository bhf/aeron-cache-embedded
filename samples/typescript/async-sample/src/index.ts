import {AeronCacheClient, EmbeddedAeronCache, EmbeddedCounterCache} from '@aeron-cache/embedded-client';

async function main() {
    const baseUrl = process.argv[2] || 'http://localhost:7070';
    const wsUrl = process.argv[3] || 'http://localhost:7071';

    console.log(`Starting Async Sample against ${baseUrl}, wsUrl: ${wsUrl}`);

    const cacheClient = new AeronCacheClient(baseUrl, wsUrl);

    try {
        const response = await cacheClient.createCache('streaming-sample-cache');
        console.log(`Created cache: ${response.cacheId}`);
    } catch (e) {
        console.error(e);
    }

    const cache = new EmbeddedAeronCache(cacheClient, 'streaming-sample-cache');
    try {
        console.log("Putting key 'streaming-key' -> 'async-value' asynchronously");
        const putResponse = await cache.put('streaming-key', 'async-value');
        console.log(`Put operation completed with status: ${putResponse.operationStatus}`);

        // Allow some time for propagation
        await new Promise(r => setTimeout(r, 100));
        const getResponse = await cache.get('streaming-key');
        console.log(`Read key 'streaming-key': ${getResponse.value || 'not found'}`);

        console.log("Putting key 'timed-key' -> 'timed-value' with 5000ms TTL");
        const timedResponse = await cache.putTimed('timed-key', 'timed-value', 5000);
        console.log(`Timed Put operation completed with status: ${timedResponse.operationStatus}`);

        const timedGet = await cache.get('timed-key');
        console.log(`Read key 'timed-key': ${timedGet.value || 'not found'}`);
    } catch (err) {
        console.error(err);
    }

    // --- Counter operations ---
    try {
        const createCounter = await cacheClient.createCounterCache('async-counter-cache');
        console.log(`Created counter cache: ${createCounter.cacheId}`);
    } catch (e) {
        console.error(e);
    }

    const counters = new EmbeddedCounterCache(cacheClient, 'async-counter-cache');
    try {
        console.log("Putting counter 'requests' -> 10");
        await counters.put('requests', 10);
        console.log(`Incremented 'requests' by 5 -> ${(await counters.increment('requests', 5)).value}`);
        console.log(`Decremented 'requests' by 3 -> ${(await counters.decrement('requests', 3)).value}`);
        console.log(`Set 'requests' -> ${(await counters.set('requests', 100)).value}`);
        console.log(`Read counter 'requests': ${(await counters.get('requests')).value}`);
    } catch (err) {
        console.error(err);
    }

    // --- Inspection & management operations ---
    try {
        console.log("Patching 'doc' (deep-merge)");
        await cacheClient.putItem('streaming-sample-cache', 'doc', '{"a":1}');
        const patchResp = await cacheClient.patchItem('streaming-sample-cache', 'doc', '{"b":2}');
        console.log(`Patch status: ${patchResp.operationStatus}`);
        console.log(`Doc after patch: ${(await cacheClient.getItem('streaming-sample-cache', 'doc')).value}`);

        console.log('Listing all caches:');
        for (const details of await cacheClient.getCaches()) {
            console.log(`  - ${details.cacheId} (${details.itemCount} items)`);
        }

        const stats = await cacheClient.getStats();
        console.log(`Cache stats: caches=${stats.totalCachesCount} items=${stats.totalItemsCount} ops=${stats.totalOpsCount} errors=${stats.errorCount}`);

        console.log("Listing all counters in 'async-counter-cache':");
        for (const item of (await cacheClient.getCounterItems('async-counter-cache')).items) {
            console.log(`  - ${item.key} = ${item.value}`);
        }

        const counterStats = await cacheClient.getCounterStats();
        console.log(`Counter stats: caches=${counterStats.totalCachesCount} items=${counterStats.totalItemsCount}`);
    } catch (err) {
        console.error(err);
    }
}

main();
