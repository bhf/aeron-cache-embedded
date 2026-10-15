"""Accumulates per-operation counts and periodic client-memory samples over a soak run, and writes a
JSON report at the end. The resolved seed and config are recorded so any finding is reproducible, and
the memory samples are the primary leak signal: with a bounded key space, resident memory that trends
upward with wall-clock time (not with the data set) is a genuine client-side leak.

Unlike the JVM soak (which samples heap via ``Runtime``), the Python client has significant work in C
extensions (``requests``, ``websockets``), so we sample process resident set size (RSS) from
``/proc/self/statm`` — the most faithful cross-cutting "used memory" signal on the Linux CI runner —
falling back to ``resource.getrusage`` where ``/proc`` is unavailable.
"""
import json
import os
import resource
import time
from collections import OrderedDict
from pathlib import Path


def _used_memory_bytes() -> int:
    """Resident set size of this process, in bytes."""
    try:
        with open("/proc/self/statm", "r") as f:
            resident_pages = int(f.read().split()[1])
        return resident_pages * os.sysconf("SC_PAGE_SIZE")
    except (OSError, IndexError, ValueError):
        # ru_maxrss is in kilobytes on Linux; a monotonic high-water mark, but still a usable fallback.
        return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024


class SoakReport:
    def __init__(self, seed: int, duration_seconds: int, config: dict) -> None:
        self._seed = seed
        self._duration_seconds = duration_seconds
        self._config = config
        self._start_millis = int(time.time() * 1000)
        self._op_counts: "OrderedDict[str, int]" = OrderedDict()
        self._total = 0
        self._samples: list[tuple[int, int]] = []  # (elapsed_millis, used_bytes)

    def record_count(self, op: str) -> None:
        self._op_counts[op] = self._op_counts.get(op, 0) + 1
        self._total += 1

    def total(self) -> int:
        return self._total

    def sample_heap(self) -> None:
        elapsed = int(time.time() * 1000) - self._start_millis
        self._samples.append((elapsed, _used_memory_bytes()))

    def progress_line(self) -> str:
        used = self._samples[-1][1] if self._samples else 0
        elapsed = (int(time.time() * 1000) - self._start_millis) // 1000
        return f"ops={self._total} elapsed={elapsed}s rssUsedMB={used // (1024 * 1024)}"

    def write_to(self, path: str) -> None:
        self.sample_heap()
        root = OrderedDict()
        root["seed"] = self._seed
        root["durationSeconds"] = self._duration_seconds
        root["config"] = self._config
        root["startMillis"] = self._start_millis
        root["endMillis"] = int(time.time() * 1000)
        root["totalOps"] = self._total
        root["opCounts"] = dict(self._op_counts)
        root["heapSamples"] = [
            {"elapsedMillis": e, "usedHeapBytes": u} for (e, u) in self._samples
        ]
        out = Path(path)
        out.parent.mkdir(parents=True, exist_ok=True)
        with out.open("w") as f:
            json.dump(root, f, indent=2)
