import { EmbeddedAeronCache } from '@aeron-cache/embedded-client';

async function main() {
    const baseUrl = process.argv[2] || 'http://localhost:8080';
    console.log(`Starting Streaming Sample against ${baseUrl}`);

    const cache = new EmbeddedAeronCache(baseUrl);
    try {
        await cache.connect();
        console.log("Connected. Waiting for updates on 'shared-key'...");

        let lastValue = "";
        
        // Loop forever checking for updates
        setInterval(() => {
            const current = cache.get('shared-key');
            if (current !== undefined && current !== lastValue) {
                console.log(`Observed change in 'shared-key': ${current}`);
                lastValue = current;
            }
        }, 1000);

        // Keep process alive
        await new Promise(() => {});

    } catch (err) {
        console.error(err);
        cache.close();
    }
}

main();
