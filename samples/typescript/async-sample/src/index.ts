import { EmbeddedAeronCache } from '@aeron-cache/embedded-client';

async function main() {
    const baseUrl = process.argv[2] || 'http://localhost:8080';
    console.log(`Starting Async Sample against ${baseUrl}`);

    const cache = new EmbeddedAeronCache(baseUrl);
    try {
        await cache.connect();

        console.log("Putting key 'async-key' -> 'async-value' asynchronously");
        
        // Fire off the put
        const putPromise = cache.put('async-key', 'async-value');
        
        // Wait for it
        await putPromise;
        console.log("Put operation completed.");

        // Allow some time for propagation
        await new Promise(r => setTimeout(r, 100));

        const val = cache.get('async-key');
        console.log(`Read key 'async-key': ${val || 'not found'}`);

    } catch (err) {
        console.error(err);
    } finally {
        cache.close();
    }
}

main();
