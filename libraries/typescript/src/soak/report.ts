import * as fs from 'fs';
import * as path from 'path';

/**
 * Accumulates per-operation counts and periodic client-heap samples over a soak run, and writes a
 * JSON report at the end. The resolved seed and config are recorded so any finding is reproducible,
 * and the heap samples are the primary leak signal: with a bounded key space, used heap that trends
 * upward with wall-clock time (not with the data set) is a genuine client-side leak.
 */
export class SoakReport {
    private readonly startMillis = Date.now();
    private readonly opCounts = new Map<string, number>();
    private totalOps = 0;
    private readonly heapSamples: Array<[number, number]> = []; // [elapsedMillis, usedBytes]

    constructor(
        private readonly seed: number,
        private readonly durationSeconds: number,
        private readonly config: Record<string, unknown>,
    ) {}

    recordCount(op: string): void {
        this.opCounts.set(op, (this.opCounts.get(op) ?? 0) + 1);
        this.totalOps++;
    }

    total(): number {
        return this.totalOps;
    }

    sampleHeap(): void {
        this.heapSamples.push([Date.now() - this.startMillis, process.memoryUsage().heapUsed]);
    }

    progressLine(): string {
        const used = this.heapSamples.length === 0 ? 0 : this.heapSamples[this.heapSamples.length - 1][1];
        return `ops=${this.totalOps}`
            + ` elapsed=${Math.floor((Date.now() - this.startMillis) / 1000)}s`
            + ` heapUsedMB=${Math.floor(used / (1024 * 1024))}`;
    }

    writeTo(filePath: string): void {
        this.sampleHeap();
        const counts: Record<string, number> = {};
        for (const [k, v] of this.opCounts) {
            counts[k] = v;
        }
        const root = {
            seed: this.seed,
            durationSeconds: this.durationSeconds,
            config: this.config,
            startMillis: this.startMillis,
            endMillis: Date.now(),
            totalOps: this.totalOps,
            opCounts: counts,
            heapSamples: this.heapSamples.map(([elapsedMillis, usedHeapBytes]) => ({
                elapsedMillis,
                usedHeapBytes,
            })),
        };
        fs.mkdirSync(path.dirname(filePath), { recursive: true });
        fs.writeFileSync(filePath, JSON.stringify(root, null, 2));
    }
}
