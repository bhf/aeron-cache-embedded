package com.bhf.aeroncache.client;

import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.fail;

/**
 * Guard against false-green CI runs.
 * <p>
 * Every integration suite in this project is gated on {@code AERON_CACHE_BASE_URL} and skips (green)
 * when it is unset. That means a dropped env var or a dead/half-started backend would let CI pass
 * without a single integration test actually exercising the backend.
 * <p>
 * This canary closes that hole. It is NOT gated on {@code AERON_CACHE_BASE_URL}; instead it keys off
 * {@code AERON_CACHE_REQUIRE_BACKEND}, which CI sets whenever a live backend is expected. When that
 * flag is set, the backend MUST be reachable or this test fails — turning a silent skip into a hard
 * failure. Locally (flag unset) it is skipped, so the normal dev workflow is unchanged.
 */
class BackendRequiredIntegrationTest {

    @Test
    void backendIsReachableWhenRequired() throws Exception {
        final String flag = System.getenv("AERON_CACHE_REQUIRE_BACKEND");
        final boolean required = flag != null
                && switch (flag.trim().toLowerCase()) {
                    case "1", "true", "yes" -> true;
                    default -> false;
                };
        Assumptions.assumeTrue(required,
                "AERON_CACHE_REQUIRE_BACKEND not set; a live backend is not required in this environment");

        final String baseUrl = System.getenv("AERON_CACHE_BASE_URL");
        assertFalse(baseUrl == null || baseUrl.isBlank(),
                "AERON_CACHE_REQUIRE_BACKEND is set but AERON_CACHE_BASE_URL is empty — the integration "
                        + "suites would silently skip. The CI backend env is misconfigured.");

        final String url = baseUrl.replaceAll("/+$", "") + "/api/v1/caches";
        final HttpClient httpClient = HttpClient.newBuilder()
                .connectTimeout(Duration.ofSeconds(10))
                .build();
        final HttpRequest request = HttpRequest.newBuilder(URI.create(url))
                .timeout(Duration.ofSeconds(10))
                .GET()
                .build();

        final HttpResponse<Void> response;
        try {
            response = httpClient.send(request, HttpResponse.BodyHandlers.discarding());
        } catch (Exception e) {
            fail("AERON_CACHE_REQUIRE_BACKEND is set but the backend at " + url + " is unreachable: " + e
                    + ". Integration tests would have silently skipped.");
            return;
        }
        assertEquals(200, response.statusCode(), "backend at " + url + " returned HTTP " + response.statusCode());
    }
}
