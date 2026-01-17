import WebSocket from 'ws';

// Polyfill global WebSocket for Node.js environment
// @ts-ignore
global.WebSocket = WebSocket;

import {AeronCacheClient, EmbeddedAeronCache} from '@aeron-cache/embedded-client';

async function main() {
    const baseUrl = process.argv[2] || 'http://localhost:7070';
    const wsUrl = process.argv[3] || 'http://localhost:7071';

    console.log(`Starting Streaming Sample against ${baseUrl}, wsUrl: ${wsUrl}`);

    const cacheClient = new AeronCacheClient(baseUrl, wsUrl);

    try {
        await cacheClient.createCache('streaming-sample-cache');
    } catch (e) {
        console.error(e);
    }

    const cache = new EmbeddedAeronCache(cacheClient, 'streaming-sample-cache');
    const onMessage = () => {
        console.info('Got message from Aeron Cache')
    };
    const onError = () => {
        console.error('Error in subscription');
    };

    cache.subscribe(onMessage, onError);
    try {
        console.log("Connected. Waiting for updates on 'shared-key'...");

        // Loop forever checking for updates
        setInterval(async () => {
            try {
                const current = await cache.getLocal('shared-key');
                if (current !== undefined) {
                    const result = JSON.stringify(current);
                    console.log(`Read key 'shared-key': ${result}`);
                }
            } catch (e) {
            }
        }, 1000);

        // Keep process alive
        await new Promise(() => {});

    } catch (err) {
        console.error(err);
    }
}

main();
