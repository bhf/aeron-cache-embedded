import { SoakClient } from './clients';
import { SoakConfig, configToReportMap } from './config';
import { SoakOracle } from './oracle';
import { SoakReport } from './report';
import { SoakRandom } from './rng';

enum OpType {
    KV_PUT = 'KV_PUT',
    KV_GET = 'KV_GET',
    KV_REMOVE = 'KV_REMOVE',
    KV_CLEAR = 'KV_CLEAR',
    KV_DELETE_RECREATE = 'KV_DELETE_RECREATE',
    COUNTER_PUT = 'COUNTER_PUT',
    COUNTER_INC = 'COUNTER_INC',
    COUNTER_DEC = 'COUNTER_DEC',
    COUNTER_SET = 'COUNTER_SET',
    COUNTER_GET = 'COUNTER_GET',
}

const ALPHABET = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
const PROGRESS_EVERY = 5000;
const SUCCESS = 'SUCCESS';

/**
 * The single-writer, time-bounded soak workload (a faithful port of the Java soak).
 *
 * One async loop drives a weighted, seeded-random mix of key-value and counter operations through a
 * {@link SoakClient} against a bounded key space spread over a handful of caches, keeping a
 * {@link SoakOracle} model in lock-step with every applied mutation. Each operation is verified
 * immediately where the backend echoes a value (gets, counter inc/dec/set); the full model is
 * reconciled against the backend every {@link SoakConfig.verifyEvery} operations and once more at the
 * end. Any divergence, non-{@code SUCCESS} status, thrown error, or operation exceeding
 * {@link SoakConfig.opTimeoutSeconds} fails the run with the seed logged for reproduction.
 */
export class SoakWorkload {
    private readonly report: SoakReport;
    private readonly oracle = new SoakOracle();
    private readonly rnd: SoakRandom;
    private readonly opTimeoutMillis: number;
    private readonly opTable: OpType[];

    private readonly kvCacheIds: string[] = [];
    private readonly counterCacheIds: string[] = [];

    constructor(private readonly client: SoakClient, private readonly cfg: SoakConfig) {
        this.report = new SoakReport(cfg.seed, cfg.durationSeconds, configToReportMap(cfg));
        this.rnd = new SoakRandom(cfg.seed);
        this.opTimeoutMillis = cfg.opTimeoutSeconds * 1000;
        this.opTable = this.buildOpTable(cfg);
    }

    getReport(): SoakReport {
        return this.report;
    }

    async run(): Promise<void> {
        console.log('[soak] starting ' + JSON.stringify({ ...configToReportMap(this.cfg), seed: this.cfg.seed, durationSeconds: this.cfg.durationSeconds }));
        try {
            await this.setupCaches();
            const deadline = Date.now() + this.cfg.durationSeconds * 1000;
            while (Date.now() < deadline) {
                const op = this.opTable[this.rnd.nextInt(this.opTable.length)];
                const applied = await this.execute(op);
                this.report.recordCount(applied);

                const ops = this.report.total();
                if (ops % this.cfg.verifyEvery === 0) {
                    await this.reconcileAll();
                }
                if (ops % PROGRESS_EVERY === 0) {
                    this.report.sampleHeap();
                    console.log('[soak] ' + this.report.progressLine());
                }
            }
            await this.reconcileAll(); // end on a verified-consistent state
            console.log('[soak] complete: ' + this.report.progressLine());
        } catch (t) {
            // Make every failure reproducible: the seed replays the exact op sequence.
            const msg = t instanceof Error ? t.message : String(t);
            throw new Error(`Soak failed (seed=${this.cfg.seed}, transport=${this.cfg.transport}, opsSoFar=${this.report.total()}): ${msg}`);
        }
    }

    // ----------------------------------------------------------------- setup

    private async setupCaches(): Promise<void> {
        for (let i = 0; i < this.cfg.kvCacheCount; i++) {
            const id = `soak-${this.cfg.transport}-kv-${i}`;
            await this.client.createKvCache(id);
            this.oracle.registerKvCache(id);
            this.kvCacheIds.push(id);
        }
        for (let i = 0; i < this.cfg.counterCacheCount; i++) {
            const id = `soak-${this.cfg.transport}-ctr-${i}`;
            await this.client.createCounterCache(id);
            this.oracle.registerCounterCache(id);
            this.counterCacheIds.push(id);
        }
    }

    // ----------------------------------------------------------------- ops

    /** @returns the op actually applied (an op may fall back when its precondition is unmet). */
    private async execute(op: OpType): Promise<OpType> {
        switch (op) {
            case OpType.KV_PUT: {
                const cache = this.randomKv(), key = this.randomKey(), val = this.randomValue();
                this.check(SUCCESS === await this.timed(() => this.client.put(cache, key, val)), () => `put !SUCCESS ${cache}/${key}`);
                this.oracle.putKv(cache, key, val);
                return OpType.KV_PUT;
            }
            case OpType.KV_GET: {
                const cache = this.randomKv(), key = this.randomKey();
                const actual = await this.timed(() => this.client.get(cache, key));
                const expected = this.oracle.getKv(cache, key);
                this.check(actual === expected,
                    () => `kv get mismatch ${cache}/${key} expected=${expected} actual=${actual}`);
                return OpType.KV_GET;
            }
            case OpType.KV_REMOVE: {
                const cache = this.randomKv();
                const keys = this.oracle.kvKeys(cache);
                if (keys.length === 0) {
                    return this.execute(OpType.KV_PUT);
                }
                const key = keys[this.rnd.nextInt(keys.length)];
                this.check(SUCCESS === await this.timed(() => this.client.remove(cache, key)), () => `remove !SUCCESS ${cache}/${key}`);
                this.oracle.removeKv(cache, key);
                return OpType.KV_REMOVE;
            }
            case OpType.KV_CLEAR: {
                const cache = this.randomKv();
                this.check(SUCCESS === await this.timed(() => this.client.clear(cache)), () => `clear !SUCCESS ${cache}`);
                this.oracle.clearKv(cache);
                return OpType.KV_CLEAR;
            }
            case OpType.KV_DELETE_RECREATE: {
                const cache = this.randomKv();
                this.check(SUCCESS === await this.timed(() => this.client.deleteCache(cache)), () => `deleteCache !SUCCESS ${cache}`);
                await this.timed(async () => { await this.client.createKvCache(cache); return null; });
                this.oracle.clearKv(cache);
                return OpType.KV_DELETE_RECREATE;
            }
            case OpType.COUNTER_PUT: {
                // putCounter is the counter creator (set/inc/dec require the key to already exist).
                const cache = this.randomCounter(), key = this.randomKey();
                const value = this.rnd.nextInt(1000);
                this.check(SUCCESS === await this.timed(() => this.client.putCounter(cache, key, value)),
                    () => `putCounter !SUCCESS ${cache}/${key}`);
                this.oracle.setCounter(cache, key, value);
                await this.verifyCounter(cache, key, value);
                return OpType.COUNTER_PUT;
            }
            case OpType.COUNTER_INC:
            case OpType.COUNTER_DEC: {
                const cache = this.randomCounter();
                const key = this.pickPresentCounterKey(cache);
                if (key === null) {
                    return this.execute(OpType.COUNTER_PUT); // seed a counter first
                }
                const amount = 1 + this.rnd.nextInt(10);
                const delta = op === OpType.COUNTER_INC ? amount : -amount;
                await this.timed(() => op === OpType.COUNTER_INC
                    ? this.client.incrementCounter(cache, key, amount)
                    : this.client.decrementCounter(cache, key, amount));
                const expected = this.oracle.addToCounter(cache, key, delta);
                // Verify via read-back rather than the mutation's echoed value (which varies by transport).
                await this.verifyCounter(cache, key, expected);
                return op;
            }
            case OpType.COUNTER_SET: {
                const cache = this.randomCounter();
                const key = this.pickPresentCounterKey(cache);
                if (key === null) {
                    return this.execute(OpType.COUNTER_PUT);
                }
                const value = this.rnd.nextInt(1000);
                await this.timed(() => this.client.setCounter(cache, key, value));
                this.oracle.setCounter(cache, key, value);
                await this.verifyCounter(cache, key, value);
                return OpType.COUNTER_SET;
            }
            case OpType.COUNTER_GET: {
                const cache = this.randomCounter();
                const key = this.pickPresentCounterKey(cache);
                if (key === null) {
                    return this.execute(OpType.COUNTER_PUT);
                }
                await this.verifyCounter(cache, key, this.oracle.getCounter(cache, key));
                return OpType.COUNTER_GET;
            }
            default:
                throw new Error('unhandled op ' + op);
        }
    }

    // ----------------------------------------------------------------- reconciliation

    private async reconcileAll(): Promise<void> {
        for (const cache of this.kvCacheIds) {
            const expected = this.oracle.kvSnapshot(cache);
            const actual = await this.client.listKv(cache);
            this.check(mapsEqual(expected, actual),
                () => `kv reconcile mismatch ${cache} expectedSize=${expected.size} actualSize=${actual.size} diff=${firstDiff(expected, actual)}`);
        }
        // Counters reconcile per key over the bounded key space (inline inc/dec/set verification
        // already covers value drift continuously).
        for (const cache of this.counterCacheIds) {
            for (const key of this.oracle.counterKeys(cache)) {
                const expected = this.oracle.getCounter(cache, key);
                const actual = await this.client.getCounter(cache, key);
                this.check(actual === expected,
                    () => `counter reconcile mismatch ${cache}/${key} expected=${expected} actual=${actual}`);
            }
        }
    }

    // ----------------------------------------------------------------- helpers

    private randomKv(): string {
        return this.kvCacheIds[this.rnd.nextInt(this.kvCacheIds.length)];
    }

    private randomCounter(): string {
        return this.counterCacheIds[this.rnd.nextInt(this.counterCacheIds.length)];
    }

    private randomKey(): string {
        return 'k' + this.rnd.nextInt(this.cfg.keySpace);
    }

    private async verifyCounter(cache: string, key: string, expected: number | null): Promise<void> {
        const actual = await this.timed(() => this.client.getCounter(cache, key));
        this.check(actual === expected,
            () => `counter mismatch ${cache}/${key} expected=${expected} actual=${actual}`);
    }

    private pickPresentCounterKey(cache: string): string | null {
        const keys = this.oracle.counterKeys(cache);
        return keys.length === 0 ? null : keys[this.rnd.nextInt(keys.length)];
    }

    private randomValue(): string {
        let s = '';
        for (let i = 0; i < this.cfg.valueSizeBytes; i++) {
            s += ALPHABET[this.rnd.nextInt(ALPHABET.length)];
        }
        return s;
    }

    private async timed<T>(op: () => Promise<T>): Promise<T> {
        const start = Date.now();
        let timer: NodeJS.Timeout | undefined;
        const timeout = new Promise<never>((_, reject) => {
            timer = setTimeout(
                () => reject(new Error(`operation exceeded ${this.cfg.opTimeoutSeconds}s`)),
                this.opTimeoutMillis);
        });
        try {
            const result = await Promise.race([op(), timeout]);
            const elapsed = Date.now() - start;
            if (elapsed > this.opTimeoutMillis) {
                throw new Error(`operation exceeded ${this.cfg.opTimeoutSeconds}s (${elapsed}ms)`);
            }
            return result;
        } finally {
            if (timer) {
                clearTimeout(timer);
            }
        }
    }

    private check(condition: boolean, message: () => string): void {
        if (!condition) {
            throw new Error(message());
        }
    }

    private buildOpTable(cfg: SoakConfig): OpType[] {
        const table: OpType[] = [];
        if (cfg.kvCacheCount > 0) {
            addWeighted(table, OpType.KV_PUT, 5);
            addWeighted(table, OpType.KV_GET, 5);
            addWeighted(table, OpType.KV_REMOVE, 2);
            addWeighted(table, OpType.KV_CLEAR, 1);
            addWeighted(table, OpType.KV_DELETE_RECREATE, 1);
        }
        if (cfg.counterCacheCount > 0) {
            addWeighted(table, OpType.COUNTER_PUT, 3);
            addWeighted(table, OpType.COUNTER_INC, 4);
            addWeighted(table, OpType.COUNTER_DEC, 2);
            addWeighted(table, OpType.COUNTER_SET, 2);
            addWeighted(table, OpType.COUNTER_GET, 4);
        }
        if (table.length === 0) {
            throw new Error('soak needs at least one kv or counter cache');
        }
        return table;
    }
}

function addWeighted(table: OpType[], op: OpType, weight: number): void {
    for (let i = 0; i < weight; i++) {
        table.push(op);
    }
}

function mapsEqual(a: Map<string, string>, b: Map<string, string>): boolean {
    if (a.size !== b.size) {
        return false;
    }
    for (const [k, v] of a) {
        if (b.get(k) !== v) {
            return false;
        }
    }
    return true;
}

function firstDiff(expected: Map<string, string>, actual: Map<string, string>): string {
    for (const [k, v] of expected) {
        if (actual.get(k) !== v) {
            return `key=${k} expected=${v} actual=${actual.get(k)}`;
        }
    }
    for (const k of actual.keys()) {
        if (!expected.has(k)) {
            return `spurious key=${k} actual=${actual.get(k)}`;
        }
    }
    return '(none)';
}
