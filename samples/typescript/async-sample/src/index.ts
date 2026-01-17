import {AeronCacheClient, EmbeddedAeronCache} from '@aeron-cache/embedded-client';

async function main() {
    const baseUrl = process.argv[2] || 'http://localhost:7070';
    const wsUrl = process.argv[3] || 'http://localhost:7071';

    console.log(`Starting Async Sample against ${baseUrl}, wsUrl: ${wsUrl}`);

    const cacheClient = new AeronCacheClient(baseUrl, wsUrl);

    try {
        await cacheClient.createCache('async-sample-cache');
    } catch (e) {
        console.error(e);
    }

    const cache = new EmbeddedAeronCache(cacheClient, 'async-sample-cache');
    try {
        console.log("Putting key 'async-key' -> 'async-value' asynchronously");
        const putPromise = await cache.put('async-key', 'async-value');
        console.log("Put operation completed.");

        // Allow some time for propagation
        await new Promise(r => setTimeout(r, 100));
        const val = await cache.get('async-key');
        const json = JSON.stringify(val);
        console.log(`Read key 'async-key': ${json || 'not found'}`);
    } catch (err) {
        console.error(err);
    }
}

main();
