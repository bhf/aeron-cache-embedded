/**
 * Configuration for a client soak run, resolved from {@code SOAK_*} environment variables (set by the
 * {@code Soak TypeScript} workflow's dispatch inputs) with sensible defaults mirroring the Java soak.
 * The resolved values — crucially the {@link seed} — are recorded in the report so any failure is
 * reproducible by re-running with the same {@code SOAK_SEED}.
 *
 * The run is purely time-bounded: the workload drives a weighted, seeded-random mix of operations
 * until {@link durationSeconds} of wall-clock has elapsed. Because the key space is bounded, the data
 * held by both the backend and the oracle is bounded too, so client heap that grows with <em>time</em>
 * rather than with the key space is a genuine leak — which is the point of the soak.
 */
export interface SoakConfig {
    /** Which client transport to drive: {@code http} or {@code bidi}. */
    transport: string;
    /** Total wall-clock duration to drive load for. */
    durationSeconds: number;
    /** PRNG seed; every random choice derives from it, so a run is reproducible given the seed. */
    seed: number;
    /** Number of key-value caches the workload spreads load across. */
    kvCacheCount: number;
    /** Number of counter caches the workload spreads load across. */
    counterCacheCount: number;
    /** Size of the key space per cache; a small space forces overwrites, removes and collisions. */
    keySpace: number;
    /** Size in bytes of randomly generated values. */
    valueSizeBytes: number;
    /** Run a full oracle reconciliation sweep every N operations. */
    verifyEvery: number;
    /** Per-operation timeout; a breach means the backend/client stalled — a genuine soak finding. */
    opTimeoutSeconds: number;
}

function strEnv(key: string, def: string): string {
    const v = process.env[key];
    return v === undefined || v.trim() === '' ? def : v.trim();
}

function intEnv(key: string, def: number): number {
    const v = process.env[key];
    return v === undefined || v.trim() === '' ? def : parseInt(v.trim(), 10);
}

/** A blank or missing seed yields a fresh random seed (recorded in the report for reproduction). */
function seedEnv(key: string): number {
    const v = process.env[key];
    if (v === undefined || v.trim() === '') {
        return (Math.floor(Math.random() * 0x100000000)) >>> 0;
    }
    return parseInt(v.trim(), 10) >>> 0;
}

export function configFromEnv(): SoakConfig {
    return {
        transport: strEnv('SOAK_TRANSPORT', 'http'),
        durationSeconds: intEnv('SOAK_DURATION_SECONDS', 1800),
        seed: seedEnv('SOAK_SEED'),
        kvCacheCount: intEnv('SOAK_KV_CACHE_COUNT', 4),
        counterCacheCount: intEnv('SOAK_COUNTER_CACHE_COUNT', 4),
        keySpace: intEnv('SOAK_KEY_SPACE', 500),
        valueSizeBytes: intEnv('SOAK_VALUE_SIZE_BYTES', 160),
        verifyEvery: intEnv('SOAK_VERIFY_EVERY', 2000),
        opTimeoutSeconds: intEnv('SOAK_OP_TIMEOUT_SECONDS', 30),
    };
}

/** Ordered config fields for the run report (seed and duration are recorded separately). */
export function configToReportMap(cfg: SoakConfig): Record<string, unknown> {
    return {
        transport: cfg.transport,
        kvCacheCount: cfg.kvCacheCount,
        counterCacheCount: cfg.counterCacheCount,
        keySpace: cfg.keySpace,
        valueSizeBytes: cfg.valueSizeBytes,
        verifyEvery: cfg.verifyEvery,
        opTimeoutSeconds: cfg.opTimeoutSeconds,
    };
}
