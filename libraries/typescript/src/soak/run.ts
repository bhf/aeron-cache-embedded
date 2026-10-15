import { configFromEnv, SoakConfig } from './config';
import { BidiSoakClient, HttpSoakClient, SoakClient } from './clients';
import { SoakWorkload } from './workload';

/**
 * Entry point for the TypeScript client soak. Drives one transport ({@code http} or {@code bidi},
 * selected by {@code SOAK_TRANSPORT}) against a running backend, reconciling against an in-memory
 * oracle and sampling client heap throughout. Correctness drift, a stalled op, or heap that trends up
 * with time are soak findings. Run via the {@code Soak TypeScript} workflow.
 *
 * Mirrors the Java {@code CacheSoakTest}: HTTP/bidi require {@code AERON_CACHE_BASE_URL}/
 * {@code AERON_CACHE_WS_URL} and skip (exit 0) when they are unset (no backend).
 */
async function main(): Promise<void> {
    const cfg = configFromEnv();

    const baseUrl = process.env.AERON_CACHE_BASE_URL;
    let wsUrl = process.env.AERON_CACHE_WS_URL;
    if (baseUrl && !wsUrl) {
        wsUrl = baseUrl.replace('http://', 'ws://').replace('https://', 'wss://');
    }

    if (!baseUrl || !wsUrl) {
        console.log('[soak] skipped: AERON_CACHE_BASE_URL / AERON_CACHE_WS_URL not set (no backend)');
        return;
    }

    const client = await buildClient(cfg, baseUrl, wsUrl);
    try {
        const workload = new SoakWorkload(client, cfg);
        await workload.run();
        workload.getReport().writeTo(`reports/soak/soak-${cfg.transport}.json`);
    } finally {
        await client.close();
    }
}

async function buildClient(cfg: SoakConfig, baseUrl: string, wsUrl: string): Promise<SoakClient> {
    switch (cfg.transport) {
        case 'http':
            return new HttpSoakClient(baseUrl, wsUrl);
        case 'bidi': {
            const client = new BidiSoakClient(wsUrl);
            await client.connect();
            return client;
        }
        default:
            throw new Error(`unknown SOAK_TRANSPORT: ${cfg.transport} (expected http|bidi)`);
    }
}

main().then(
    () => process.exit(0),
    (err) => {
        console.error(err instanceof Error ? err.stack ?? err.message : err);
        process.exit(1);
    },
);
