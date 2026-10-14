package com.bhf.aeroncache.soak;

import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;

import java.nio.file.Path;

/**
 * Long-running client soak: drives a weighted, seeded-random mix of key-value and counter operations
 * through one client transport against a running backend, reconciling against an in-memory oracle and
 * sampling client heap throughout. Correctness drift, a stalled op, or heap that trends up with time are
 * soak findings.
 *
 * <p>Gated on {@code -Dsoak.enabled=true} (set by the {@code soakTest} Gradle task) so it never runs under
 * a normal {@code test}/{@code build}; HTTP/bidi additionally require {@code AERON_CACHE_BASE_URL}/
 * {@code AERON_CACHE_WS_URL} and skip (green) when they are unset. Transport is selected by
 * {@code -Psoak.transport} ({@code http}|{@code bidi}|{@code gateway}); run via the
 * {@code Soak Java} workflow.
 */
@DisplayName("Client cache soak")
class CacheSoakTest {

    @Test
    void soak() throws Exception {
        Assumptions.assumeTrue("true".equals(System.getProperty("soak.enabled")),
                "Soak disabled: run via `./gradlew soakTest` (sets -Dsoak.enabled=true)");

        final SoakConfig cfg = SoakConfig.fromSystemProperties();
        try (SoakClient client = buildClient(cfg)) {
            final SoakWorkload workload = new SoakWorkload(client, cfg);
            workload.run();
            workload.report().writeTo(Path.of("build/reports/soak/soak-" + cfg.transport + ".json"));
        }
    }

    private static SoakClient buildClient(SoakConfig cfg) throws Exception {
        switch (cfg.transport) {
            case "http":
                return new HttpSoakClient(requireEnv("AERON_CACHE_BASE_URL"), requireEnv("AERON_CACHE_WS_URL"));
            case "bidi":
                return new BidiSoakClient(requireEnv("AERON_CACHE_WS_URL"));
            case "gateway":
                return new GatewaySoakClient(System.getProperty("aeron.gateway.host", "127.0.0.1"));
            default:
                throw new IllegalArgumentException("unknown soak.transport: " + cfg.transport
                        + " (expected http|bidi|gateway)");
        }
    }

    private static String requireEnv(String name) {
        final String value = System.getenv(name);
        Assumptions.assumeTrue(value != null && !value.isBlank(),
                "Soak skipped: " + name + " not set (no backend)");
        return value;
    }
}
