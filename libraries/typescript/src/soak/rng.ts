/**
 * A small deterministic PRNG so a soak run is reproducible from its recorded seed. Node has no
 * seedable RNG in its standard library, so we use mulberry32 — a well-known 32-bit generator with a
 * good-enough distribution for weighted op selection and key/value fuzzing (this is a workload
 * driver, not a cryptographic context).
 */
export class SoakRandom {
    private state: number;

    constructor(seed: number) {
        // Fold to an unsigned 32-bit state; any 32-bit seed yields a distinct, repeatable stream.
        this.state = seed >>> 0;
    }

    /** Next float in [0, 1). */
    nextFloat(): number {
        this.state = (this.state + 0x6d2b79f5) | 0;
        let t = this.state;
        t = Math.imul(t ^ (t >>> 15), t | 1);
        t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    }

    /** Next int in [0, bound). */
    nextInt(bound: number): number {
        return Math.floor(this.nextFloat() * bound);
    }
}
