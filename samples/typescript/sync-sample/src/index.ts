import { EmbeddedAeronCache } from '@aeron-cache/embedded-client';

async function main() {
    const baseUrl = process.argv[2] || 'http://localhost:8080';
    console.log(`Starting Sync Sample against ${baseUrl}`);

    const cache = new EmbeddedAeronCache(baseUrl);
    try {
        await cache.connect();

        console.log("Putting key 'sync-key' -> 'sync-value'");
        await cache.put('sync-key', 'sync-value');

        // Allow some time for WebSocket propagation
        await new Promise(r => setTimeout(r, 100));

        const val = cache.get('sync-key');
        if (val) {
            console.log(`Read back key 'sync-key': ${val}`);
        } else {
            console.log("Key 'sync-key' not found in local cache yet.");
        }

    } catch (err) {
        console.error(err);
    } finally {
        cache.close();
    }
}

main();
