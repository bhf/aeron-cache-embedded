//! Guard against false-green CI runs.
//!
//! Every integration suite in this crate is gated on `AERON_CACHE_BASE_URL` and early-returns
//! (passing) when it is unset. That means a dropped env var or a dead/half-started backend would
//! let CI pass without a single integration test actually exercising the backend.
//!
//! This canary closes that hole. It is NOT gated on `AERON_CACHE_BASE_URL`; instead it keys off
//! `AERON_CACHE_REQUIRE_BACKEND`, which CI sets whenever a live backend is expected. When that flag
//! is set, the backend MUST be reachable or this test panics — turning a silent skip into a hard
//! failure. Locally (flag unset) it no-ops, so the normal dev workflow is unchanged.

use std::env;
use std::time::Duration;

fn backend_required() -> bool {
    env::var("AERON_CACHE_REQUIRE_BACKEND")
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

#[test]
fn backend_is_reachable() {
    if !backend_required() {
        eprintln!("AERON_CACHE_REQUIRE_BACKEND not set; a live backend is not required here, skipping");
        return;
    }

    let base = env::var("AERON_CACHE_BASE_URL").unwrap_or_default();
    assert!(
        !base.trim().is_empty(),
        "AERON_CACHE_REQUIRE_BACKEND is set but AERON_CACHE_BASE_URL is empty — the integration \
         suites would silently skip. The CI backend env is misconfigured."
    );

    let url = format!("{}/api/v1/caches", base.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build HTTP client");

    match client.get(&url).send() {
        Ok(response) => assert_eq!(
            response.status().as_u16(),
            200,
            "backend at {url} returned HTTP {}, expected 200",
            response.status()
        ),
        Err(err) => panic!(
            "AERON_CACHE_REQUIRE_BACKEND is set but the backend at {url} is unreachable: {err}. \
             Integration tests would have silently skipped."
        ),
    }
}
