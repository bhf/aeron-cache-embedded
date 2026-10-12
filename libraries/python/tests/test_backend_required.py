"""Guard against false-green CI runs.

Every integration suite in this project is gated on ``AERON_CACHE_BASE_URL`` and *skips*
(green) when it is unset. That means a dropped env var or a dead/half-started backend would
let CI pass without a single integration test actually running.

This canary closes that hole. It is NOT gated on ``AERON_CACHE_BASE_URL``; instead it keys off
``AERON_CACHE_REQUIRE_BACKEND``, which CI sets whenever a live backend is expected. When that
flag is set, the backend MUST be reachable or this test fails — turning a silent skip into a
hard failure. Locally (flag unset) it skips, so the normal dev workflow is unchanged.
"""

import os
import urllib.error
import urllib.request

import pytest

_REQUIRE = os.environ.get("AERON_CACHE_REQUIRE_BACKEND", "").strip().lower() in ("1", "true", "yes")


@pytest.mark.skipif(
    not _REQUIRE,
    reason="AERON_CACHE_REQUIRE_BACKEND not set; a live backend is not required in this environment",
)
def test_backend_is_reachable():
    base_url = os.environ.get("AERON_CACHE_BASE_URL", "").strip()
    assert base_url, (
        "AERON_CACHE_REQUIRE_BACKEND is set but AERON_CACHE_BASE_URL is empty — the integration "
        "suites would silently skip. The CI backend env is misconfigured."
    )
    url = base_url.rstrip("/") + "/api/v1/caches"
    try:
        with urllib.request.urlopen(url, timeout=10) as response:
            status = response.status
    except urllib.error.URLError as exc:
        pytest.fail(
            f"AERON_CACHE_REQUIRE_BACKEND is set but the backend at {url} is unreachable: {exc}. "
            "Integration tests would have silently skipped."
        )
    assert status == 200, f"backend at {url} returned HTTP {status}, expected 200"
