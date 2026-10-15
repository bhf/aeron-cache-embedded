"""Configuration for a client soak run, resolved from ``SOAK_*`` environment variables (set by the
``Soak Python`` workflow's dispatch inputs) with sensible defaults mirroring the Java soak.

The resolved values — crucially the seed — are recorded in the report so any failure is reproducible
by re-running with the same ``SOAK_SEED``. The run is purely time-bounded: the workload drives a
weighted, seeded-random mix of operations until ``duration_seconds`` of wall-clock has elapsed. Because
the key space is bounded, the data held by both the backend and the oracle is bounded too, so client
memory that grows with *time* rather than with the key space is a genuine leak — the point of the soak.
"""
import os
import random
from dataclasses import dataclass


@dataclass
class SoakConfig:
    # Which client transport to drive: "http" or "bidi".
    transport: str
    # Total wall-clock duration to drive load for.
    duration_seconds: int
    # PRNG seed; every random choice derives from it, so a run is reproducible given the seed.
    seed: int
    # Number of key-value caches the workload spreads load across.
    kv_cache_count: int
    # Number of counter caches the workload spreads load across.
    counter_cache_count: int
    # Size of the key space per cache; a small space forces overwrites, removes and collisions.
    key_space: int
    # Size in bytes of randomly generated values.
    value_size_bytes: int
    # Run a full oracle reconciliation sweep every N operations.
    verify_every: int
    # Per-operation timeout; a breach means the backend/client stalled — a genuine soak finding.
    op_timeout_seconds: int

    @classmethod
    def from_env(cls) -> "SoakConfig":
        return cls(
            transport=_str_env("SOAK_TRANSPORT", "http"),
            duration_seconds=_int_env("SOAK_DURATION_SECONDS", 1800),
            seed=_seed_env("SOAK_SEED"),
            kv_cache_count=_int_env("SOAK_KV_CACHE_COUNT", 4),
            counter_cache_count=_int_env("SOAK_COUNTER_CACHE_COUNT", 4),
            key_space=_int_env("SOAK_KEY_SPACE", 500),
            value_size_bytes=_int_env("SOAK_VALUE_SIZE_BYTES", 160),
            verify_every=_int_env("SOAK_VERIFY_EVERY", 2000),
            op_timeout_seconds=_int_env("SOAK_OP_TIMEOUT_SECONDS", 30),
        )

    def to_report_map(self) -> dict:
        """Ordered config fields for the run report (seed and duration recorded separately)."""
        return {
            "transport": self.transport,
            "kvCacheCount": self.kv_cache_count,
            "counterCacheCount": self.counter_cache_count,
            "keySpace": self.key_space,
            "valueSizeBytes": self.value_size_bytes,
            "verifyEvery": self.verify_every,
            "opTimeoutSeconds": self.op_timeout_seconds,
        }


def _str_env(key: str, default: str) -> str:
    v = os.environ.get(key)
    return default if v is None or v.strip() == "" else v.strip()


def _int_env(key: str, default: int) -> int:
    v = os.environ.get(key)
    return default if v is None or v.strip() == "" else int(v.strip())


def _seed_env(key: str) -> int:
    """A blank or missing seed yields a fresh random seed (recorded in the report for reproduction)."""
    v = os.environ.get(key)
    if v is None or v.strip() == "":
        return random.SystemRandom().getrandbits(63)
    return int(v.strip())
