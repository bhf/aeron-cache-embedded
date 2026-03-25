import {AeronCacheClient, EmbeddedAeronCache} from '@aeron-cache/embedded-client';

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
    } catch (err) {
        console.error(err);
    }
}

main();
