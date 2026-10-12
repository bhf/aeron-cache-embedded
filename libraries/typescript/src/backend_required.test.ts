/**
 * Guard against false-green CI runs.
 *
 * Every integration suite here is gated on `AERON_CACHE_BASE_URL` and skips (green) when it is
 * unset. That means a dropped env var or a dead/half-started backend would let CI pass without a
 * single integration test actually exercising the backend.
 *
 * This canary closes that hole. It is NOT gated on `AERON_CACHE_BASE_URL`; instead it keys off
 * `AERON_CACHE_REQUIRE_BACKEND`, which CI sets whenever a live backend is expected. When that flag
 * is set, the backend MUST be reachable or this test fails — turning a silent skip into a hard
 * failure. Locally (flag unset) it skips, so the normal dev workflow is unchanged.
 */

const requireBackend = ['1', 'true', 'yes'].includes(
  (process.env.AERON_CACHE_REQUIRE_BACKEND || '').trim().toLowerCase(),
);

(requireBackend ? describe : describe.skip)('backend required (CI guard)', () => {
  it('backend is reachable', async () => {
    const baseUrl = (process.env.AERON_CACHE_BASE_URL || '').trim();
    expect(baseUrl).not.toBe(
      '', // AERON_CACHE_REQUIRE_BACKEND is set but AERON_CACHE_BASE_URL is empty — suites would silently skip.
    );

    const url = `${baseUrl.replace(/\/$/, '')}/api/v1/caches`;
    const response = await fetch(url);
    expect(response.status).toBe(200);
  });
});
