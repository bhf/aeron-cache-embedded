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
        const response = await cacheClient.createCache('streaming-sample-cache');
        console.log(`Created cache: ${response.cacheId}`);
    } catch (e) {
        console.error(e);
    }

    const cache = new EmbeddedAeronCache(cacheClient, 'streaming-sample-cache');
    const onMessage = (event: any) => {
        console.info(`[TypeScript] Got message from Aeron Cache: Type ${event.type}, Key ${event.key}`);
    };
    const onError = () => {
        console.error('[TypeScript] Error in subscription');
    };

    cache.subscribe(onMessage, onError);
    try {
        console.log("Connected. Waiting for updates on 'streaming-key'...");
        
        let lastValue = "";
        // Loop forever checking for updates
        setInterval(async () => {
            try {
                const current = await cache.getLocal('streaming-key');
                if (current !== undefined && current !== lastValue) {
                    console.log(`[TypeScript-Poller] Observed change in 'streaming-key': ${current}`);
                    lastValue = current;
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
