"""Entry point for the Python client soak.

Drives one transport (``http`` or ``bidi``, selected by ``SOAK_TRANSPORT``) against a running backend,
reconciling against an in-memory oracle and sampling client memory throughout. Mirrors the Java
``CacheSoakTest``: HTTP/bidi require ``AERON_CACHE_BASE_URL`` / ``AERON_CACHE_WS_URL`` and skip (exit 0)
when they are unset (no backend).

Run via ``poetry run python -m soak``.
"""
import os
import sys

from .clients import BidiSoakClient, HttpSoakClient, SoakClient
from .config import SoakConfig
from .workload import SoakWorkload


def _build_client(cfg: SoakConfig, base_url: str, ws_url: str) -> SoakClient:
    if cfg.transport == "http":
        return HttpSoakClient(base_url, ws_url)
    if cfg.transport == "bidi":
        return BidiSoakClient(ws_url)
    raise ValueError(f"unknown SOAK_TRANSPORT: {cfg.transport} (expected http|bidi)")


def main() -> int:
    cfg = SoakConfig.from_env()

    base_url = os.environ.get("AERON_CACHE_BASE_URL")
    ws_url = os.environ.get("AERON_CACHE_WS_URL")
    if base_url and not ws_url:
        ws_url = base_url.replace("http://", "ws://").replace("https://", "wss://")

    if not base_url or not ws_url:
        print("[soak] skipped: AERON_CACHE_BASE_URL / AERON_CACHE_WS_URL not set (no backend)")
        return 0

    client = _build_client(cfg, base_url, ws_url)
    try:
        workload = SoakWorkload(client, cfg)
        workload.run()
        workload.report().write_to(f"reports/soak/soak-{cfg.transport}.json")
    finally:
        client.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
