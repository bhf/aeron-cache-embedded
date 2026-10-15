"""The single-writer, time-bounded soak workload (a faithful port of the Java soak).

One loop drives a weighted, seeded-random mix of key-value and counter operations through a
``SoakClient`` against a bounded key space spread over a handful of caches, keeping a ``SoakOracle``
model in lock-step with every applied mutation. Each operation is verified immediately where the
backend echoes a value (gets, counter inc/dec/set); the full model is reconciled against the backend
every ``verify_every`` operations and once more at the end. Any divergence, non-``SUCCESS`` status,
thrown error, or operation exceeding ``op_timeout_seconds`` fails the run with the seed logged for
reproduction.
"""
import random
import time
from enum import Enum
from typing import Callable, Optional

from .clients import SoakClient
from .config import SoakConfig
from .oracle import SoakOracle
from .report import SoakReport

_ALPHABET = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
_PROGRESS_EVERY = 5000
_SUCCESS = "SUCCESS"


class OpType(Enum):
    KV_PUT = "KV_PUT"
    KV_GET = "KV_GET"
    KV_REMOVE = "KV_REMOVE"
    KV_CLEAR = "KV_CLEAR"
    KV_DELETE_RECREATE = "KV_DELETE_RECREATE"
    COUNTER_PUT = "COUNTER_PUT"
    COUNTER_INC = "COUNTER_INC"
    COUNTER_DEC = "COUNTER_DEC"
    COUNTER_SET = "COUNTER_SET"
    COUNTER_GET = "COUNTER_GET"


class SoakFailure(AssertionError):
    pass


class SoakWorkload:
    def __init__(self, client: SoakClient, cfg: SoakConfig) -> None:
        self._client = client
        self._cfg = cfg
        self._report = SoakReport(cfg.seed, cfg.duration_seconds, cfg.to_report_map())
        self._rnd = random.Random(cfg.seed)
        self._op_timeout = float(cfg.op_timeout_seconds)
        self._op_table = self._build_op_table(cfg)
        self._oracle = SoakOracle()
        self._kv_cache_ids: list[str] = []
        self._counter_cache_ids: list[str] = []

    def report(self) -> SoakReport:
        return self._report

    def run(self) -> None:
        print(f"[soak] starting {self._cfg}")
        try:
            self._setup_caches()
            deadline = time.monotonic() + self._cfg.duration_seconds
            while time.monotonic() < deadline:
                op = self._op_table[self._rnd.randrange(len(self._op_table))]
                applied = self._execute(op)
                self._report.record_count(applied.value)

                ops = self._report.total()
                if ops % self._cfg.verify_every == 0:
                    self._reconcile_all()
                if ops % _PROGRESS_EVERY == 0:
                    self._report.sample_heap()
                    print(f"[soak] {self._report.progress_line()}")
            self._reconcile_all()  # end on a verified-consistent state
            print(f"[soak] complete: {self._report.progress_line()}")
        except Exception as t:
            # Make every failure reproducible: the seed replays the exact op sequence.
            raise SoakFailure(
                f"Soak failed (seed={self._cfg.seed}, transport={self._cfg.transport}, "
                f"opsSoFar={self._report.total()}): {t}"
            ) from t

    # ----------------------------------------------------------------- setup

    def _setup_caches(self) -> None:
        for i in range(self._cfg.kv_cache_count):
            cid = f"soak-{self._cfg.transport}-kv-{i}"
            self._client.create_kv_cache(cid)
            self._oracle.register_kv_cache(cid)
            self._kv_cache_ids.append(cid)
        for i in range(self._cfg.counter_cache_count):
            cid = f"soak-{self._cfg.transport}-ctr-{i}"
            self._client.create_counter_cache(cid)
            self._oracle.register_counter_cache(cid)
            self._counter_cache_ids.append(cid)

    # ----------------------------------------------------------------- ops

    def _execute(self, op: OpType) -> OpType:
        """Return the op actually applied (an op may fall back when its precondition is unmet)."""
        if op == OpType.KV_PUT:
            cache, key, val = self._random_kv(), self._random_key(), self._random_value()
            self._check(_SUCCESS == self._timed(lambda: self._client.put(cache, key, val)),
                        lambda: f"put !SUCCESS {cache}/{key}")
            self._oracle.put_kv(cache, key, val)
            return OpType.KV_PUT

        if op == OpType.KV_GET:
            cache, key = self._random_kv(), self._random_key()
            actual = self._timed(lambda: self._client.get(cache, key))
            expected = self._oracle.get_kv(cache, key)
            self._check(actual == expected,
                        lambda: f"kv get mismatch {cache}/{key} expected={expected} actual={actual}")
            return OpType.KV_GET

        if op == OpType.KV_REMOVE:
            cache = self._random_kv()
            keys = self._oracle.kv_keys(cache)
            if not keys:
                return self._execute(OpType.KV_PUT)
            key = keys[self._rnd.randrange(len(keys))]
            self._check(_SUCCESS == self._timed(lambda: self._client.remove(cache, key)),
                        lambda: f"remove !SUCCESS {cache}/{key}")
            self._oracle.remove_kv(cache, key)
            return OpType.KV_REMOVE

        if op == OpType.KV_CLEAR:
            cache = self._random_kv()
            self._check(_SUCCESS == self._timed(lambda: self._client.clear(cache)),
                        lambda: f"clear !SUCCESS {cache}")
            self._oracle.clear_kv(cache)
            return OpType.KV_CLEAR

        if op == OpType.KV_DELETE_RECREATE:
            cache = self._random_kv()
            self._check(_SUCCESS == self._timed(lambda: self._client.delete_cache(cache)),
                        lambda: f"deleteCache !SUCCESS {cache}")
            self._timed(lambda: self._client.create_kv_cache(cache))
            self._oracle.clear_kv(cache)
            return OpType.KV_DELETE_RECREATE

        if op == OpType.COUNTER_PUT:
            # put_counter is the counter creator (set/inc/dec require the key to already exist).
            cache, key = self._random_counter(), self._random_key()
            value = self._rnd.randrange(1000)
            self._check(_SUCCESS == self._timed(lambda: self._client.put_counter(cache, key, value)),
                        lambda: f"putCounter !SUCCESS {cache}/{key}")
            self._oracle.set_counter(cache, key, value)
            self._verify_counter(cache, key, value)
            return OpType.COUNTER_PUT

        if op in (OpType.COUNTER_INC, OpType.COUNTER_DEC):
            cache = self._random_counter()
            key = self._pick_present_counter_key(cache)
            if key is None:
                return self._execute(OpType.COUNTER_PUT)  # seed a counter first
            amount = 1 + self._rnd.randrange(10)
            delta = amount if op == OpType.COUNTER_INC else -amount
            if op == OpType.COUNTER_INC:
                self._timed(lambda: self._client.increment_counter(cache, key, amount))
            else:
                self._timed(lambda: self._client.decrement_counter(cache, key, amount))
            expected = self._oracle.add_to_counter(cache, key, delta)
            # Verify via read-back rather than the mutation's echoed value (varies by transport).
            self._verify_counter(cache, key, expected)
            return op

        if op == OpType.COUNTER_SET:
            cache = self._random_counter()
            key = self._pick_present_counter_key(cache)
            if key is None:
                return self._execute(OpType.COUNTER_PUT)
            value = self._rnd.randrange(1000)
            self._timed(lambda: self._client.set_counter(cache, key, value))
            self._oracle.set_counter(cache, key, value)
            self._verify_counter(cache, key, value)
            return OpType.COUNTER_SET

        if op == OpType.COUNTER_GET:
            cache = self._random_counter()
            key = self._pick_present_counter_key(cache)
            if key is None:
                return self._execute(OpType.COUNTER_PUT)
            self._verify_counter(cache, key, self._oracle.get_counter(cache, key))
            return OpType.COUNTER_GET

        raise RuntimeError(f"unhandled op {op}")

    # ----------------------------------------------------------------- reconciliation

    def _reconcile_all(self) -> None:
        for cache in self._kv_cache_ids:
            expected = self._oracle.kv_snapshot(cache)
            actual = self._client.list_kv(cache)
            self._check(actual == expected,
                        lambda: f"kv reconcile mismatch {cache} expectedSize={len(expected)} "
                                f"actualSize={len(actual)} diff={_first_diff(expected, actual)}")
        # Counters reconcile per key over the bounded key space (inline inc/dec/set verification
        # already covers value drift continuously).
        for cache in self._counter_cache_ids:
            for key in self._oracle.counter_keys(cache):
                expected = self._oracle.get_counter(cache, key)
                actual = self._client.get_counter(cache, key)
                self._check(actual == expected,
                            lambda: f"counter reconcile mismatch {cache}/{key} "
                                    f"expected={expected} actual={actual}")

    # ----------------------------------------------------------------- helpers

    def _random_kv(self) -> str:
        return self._kv_cache_ids[self._rnd.randrange(len(self._kv_cache_ids))]

    def _random_counter(self) -> str:
        return self._counter_cache_ids[self._rnd.randrange(len(self._counter_cache_ids))]

    def _random_key(self) -> str:
        return f"k{self._rnd.randrange(self._cfg.key_space)}"

    def _verify_counter(self, cache: str, key: str, expected: Optional[int]) -> None:
        actual = self._timed(lambda: self._client.get_counter(cache, key))
        self._check(actual == expected,
                    lambda: f"counter mismatch {cache}/{key} expected={expected} actual={actual}")

    def _pick_present_counter_key(self, cache: str) -> Optional[str]:
        keys = self._oracle.counter_keys(cache)
        return None if not keys else keys[self._rnd.randrange(len(keys))]

    def _random_value(self) -> str:
        return "".join(self._rnd.choice(_ALPHABET) for _ in range(self._cfg.value_size_bytes))

    def _timed(self, op: Callable):
        start = time.monotonic()
        result = op()
        elapsed = time.monotonic() - start
        if elapsed > self._op_timeout:
            raise SoakFailure(
                f"operation exceeded {self._cfg.op_timeout_seconds}s ({int(elapsed * 1000)}ms)")
        return result

    @staticmethod
    def _check(condition: bool, message: Callable[[], str]) -> None:
        if not condition:
            raise SoakFailure(message())

    @staticmethod
    def _build_op_table(cfg: SoakConfig) -> list[OpType]:
        table: list[OpType] = []
        if cfg.kv_cache_count > 0:
            _add_weighted(table, OpType.KV_PUT, 5)
            _add_weighted(table, OpType.KV_GET, 5)
            _add_weighted(table, OpType.KV_REMOVE, 2)
            _add_weighted(table, OpType.KV_CLEAR, 1)
            _add_weighted(table, OpType.KV_DELETE_RECREATE, 1)
        if cfg.counter_cache_count > 0:
            _add_weighted(table, OpType.COUNTER_PUT, 3)
            _add_weighted(table, OpType.COUNTER_INC, 4)
            _add_weighted(table, OpType.COUNTER_DEC, 2)
            _add_weighted(table, OpType.COUNTER_SET, 2)
            _add_weighted(table, OpType.COUNTER_GET, 4)
        if not table:
            raise ValueError("soak needs at least one kv or counter cache")
        return table


def _add_weighted(table: list[OpType], op: OpType, weight: int) -> None:
    table.extend([op] * weight)


def _first_diff(expected: dict[str, str], actual: dict[str, str]) -> str:
    for k, v in expected.items():
        if actual.get(k) != v:
            return f"key={k} expected={v} actual={actual.get(k)}"
    for k in actual:
        if k not in expected:
            return f"spurious key={k} actual={actual.get(k)}"
    return "(none)"
