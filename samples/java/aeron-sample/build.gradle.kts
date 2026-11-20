plugins {
    application
    java
    // Builds a container image for the sample without a Dockerfile or a Docker daemon, so CI can
    // publish it to GHCR with `./gradlew jib`. See release-aeron-sample.yml.
    id("com.google.cloud.tools.jib") version "3.4.4"
}

repositories {
    mavenLocal()
    mavenCentral()
}

dependencies {
    implementation("com.bhf.aeroncache:aeron-cache-embedded-client:1.0.2")
    implementation("org.slf4j:slf4j-simple:2.0.13")
}

application {
    mainClass.set("com.aeron.cache.sample.AeronSample")
    // Aeron/Agrona need these opens on modern JDKs.
    applicationDefaultJvmArgs = listOf(
        "--add-opens", "java.base/jdk.internal.misc=ALL-UNNAMED",
        "--add-opens", "java.base/java.util.zip=ALL-UNNAMED"
    )
}

tasks.named<JavaExec>("run") {
    // Gradle does not forward -D system properties from the CLI to a forked JavaExec by default, but
    // this sample's README documents `./gradlew run -Daeron.gateway.host=...` etc. — forward them.
    systemProperties(System.getProperties().entries.associate { (k, v) -> k.toString() to v.toString() })
}

// --- Container image (Jib) --------------------------------------------------
// Publishes the Aeron-gateway sample as a pullable image so downstream deployments (e.g.
// aeron-cache-patterns) can run it as an in-cluster writer against a gateway with no build.
//
// The image bakes the same --add-opens flags as `run`. There are no exposed ports: the
// AeronGatewayClient only dials OUT to the gateway (request :7075, response-control :7076) — the
// gateway creates each client's response publication lazily over an MDC control channel — so the
// client needs no inbound listener.
//
// Configure the target gateway at runtime with the JVM's standard JAVA_TOOL_OPTIONS env var, e.g.
//   JAVA_TOOL_OPTIONS=-Daeron.gateway.host=aeroncache-gateway
// The default (no override) is UDP against 127.0.0.1.
//
// The release workflow sets the destination tag with -Djib.to.image / -Djib.to.tags and authenticates
// to GHCR via `docker login` (Jib reads the Docker credential store); no Docker daemon is required.
jib {
    from {
        image = "eclipse-temurin:25-jre"
    }
    to {
        image = "ghcr.io/bhf/aeroncache-embedded-aeron-sample"
    }
    container {
        mainClass = "com.aeron.cache.sample.AeronSample"
        jvmFlags = listOf(
            "--add-opens", "java.base/jdk.internal.misc=ALL-UNNAMED",
            "--add-opens", "java.base/java.util.zip=ALL-UNNAMED"
        )
        creationTime.set("USE_CURRENT_TIMESTAMP")
    }
}
