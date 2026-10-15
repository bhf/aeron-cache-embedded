"""The in-memory model the workload keeps in lock-step with the backend: the expected contents of
every key-value and counter cache. Reconciliation compares this model against what the client reads
back; any divergence is a soak finding. The model is bounded by the configured key space, so it does
not itself grow with time.
"""
from typing import Optional


class SoakOracle:
    def __init__(self) -> None:
        self._kv: dict[str, dict[str, str]] = {}
        self._counters: dict[str, dict[str, int]] = {}

    def register_kv_cache(self, cache_id: str) -> None:
        self._kv[cache_id] = {}

    def register_counter_cache(self, cache_id: str) -> None:
        self._counters[cache_id] = {}

    # ----------------------------------------------------------------- key-value

    def put_kv(self, cache_id: str, key: str, value: str) -> None:
        self._kv[cache_id][key] = value

    def remove_kv(self, cache_id: str, key: str) -> None:
        self._kv[cache_id].pop(key, None)

    def clear_kv(self, cache_id: str) -> None:
        self._kv[cache_id].clear()

    def get_kv(self, cache_id: str, key: str) -> Optional[str]:
        return self._kv[cache_id].get(key)

    def kv_keys(self, cache_id: str) -> list[str]:
        """Keys currently present in a kv cache (a copy, safe to pick from while mutating)."""
        return list(self._kv[cache_id].keys())

    def kv_snapshot(self, cache_id: str) -> dict[str, str]:
        return dict(self._kv[cache_id])

    # ----------------------------------------------------------------- counters

    def set_counter(self, cache_id: str, key: str, value: int) -> None:
        self._counters[cache_id][key] = value

    def add_to_counter(self, cache_id: str, key: str, delta: int) -> int:
        nxt = self._counters[cache_id].get(key, 0) + delta
        self._counters[cache_id][key] = nxt
        return nxt

    def get_counter(self, cache_id: str, key: str) -> Optional[int]:
        return self._counters[cache_id].get(key)

    def counter_keys(self, cache_id: str) -> list[str]:
        return list(self._counters[cache_id].keys())
