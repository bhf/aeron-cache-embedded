/**
 * The in-memory model the workload keeps in lock-step with the backend: the expected contents of
 * every key-value and counter cache. Reconciliation compares this model against what the client reads
 * back; any divergence is a soak finding. The model is bounded by the configured key space, so it does
 * not itself grow with time.
 */
export class SoakOracle {
    private readonly kv = new Map<string, Map<string, string>>();
    private readonly counters = new Map<string, Map<string, number>>();

    registerKvCache(cacheId: string): void {
        this.kv.set(cacheId, new Map());
    }

    registerCounterCache(cacheId: string): void {
        this.counters.set(cacheId, new Map());
    }

    // ----------------------------------------------------------------- key-value

    putKv(cacheId: string, key: string, value: string): void {
        this.kv.get(cacheId)!.set(key, value);
    }

    removeKv(cacheId: string, key: string): void {
        this.kv.get(cacheId)!.delete(key);
    }

    clearKv(cacheId: string): void {
        this.kv.get(cacheId)!.clear();
    }

    getKv(cacheId: string, key: string): string | null {
        const v = this.kv.get(cacheId)!.get(key);
        return v === undefined ? null : v;
    }

    /** Keys currently present in a kv cache (a copy, safe to pick from while mutating). */
    kvKeys(cacheId: string): string[] {
        return Array.from(this.kv.get(cacheId)!.keys());
    }

    kvSnapshot(cacheId: string): Map<string, string> {
        return this.kv.get(cacheId)!;
    }

    // ----------------------------------------------------------------- counters

    setCounter(cacheId: string, key: string, value: number): void {
        this.counters.get(cacheId)!.set(key, value);
    }

    addToCounter(cacheId: string, key: string, delta: number): number {
        const next = (this.counters.get(cacheId)!.get(key) ?? 0) + delta;
        this.counters.get(cacheId)!.set(key, next);
        return next;
    }

    getCounter(cacheId: string, key: string): number | null {
        const v = this.counters.get(cacheId)!.get(key);
        return v === undefined ? null : v;
    }

    counterKeys(cacheId: string): string[] {
        return Array.from(this.counters.get(cacheId)!.keys());
    }
}
