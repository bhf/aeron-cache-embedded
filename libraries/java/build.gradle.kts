plugins {
    `java-library`
    `maven-publish`
}

group = "com.bhf.aeroncache"
version = "1.0.1"

repositories {
    mavenCentral()
}

// Configuration holding the SBE tool used to generate the gateway wire-protocol codecs.
val sbeToolConfig by configurations.creating

dependencies {
    implementation("com.fasterxml.jackson.core:jackson-databind:2.16.1")
    // Aeron transport for the SBE gateway interface. Exposed as `api` because io.aeron.Aeron
    // appears in AeronGatewayClient's public constructor.
    api("io.aeron:aeron-all:1.50.0")
    // SBE tool generates the gateway codecs at build time; the runtime pieces are also needed on the classpath.
    sbeToolConfig("uk.co.real-logic:sbe-tool:1.33.0")
    implementation("uk.co.real-logic:sbe-tool:1.33.0")
    testImplementation("org.junit.jupiter:junit-jupiter:5.10.2")
    testImplementation("org.mockito:mockito-core:5.11.0")
    testImplementation("org.mockito:mockito-junit-jupiter:5.11.0")
    testImplementation("org.wiremock:wiremock:3.5.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

// --- SBE codec generation for the Aeron gateway wire protocol ---
val generatedSbeSourceDir = layout.buildDirectory.dir("generated/sources/sbe/main/java")

sourceSets {
    main {
        java {
            srcDir(generatedSbeSourceDir)
        }
    }
}

tasks.register<JavaExec>("generateSbeCodecs") {
    mainClass.set("uk.co.real_logic.sbe.SbeTool")
    classpath = sbeToolConfig
    // Single source of truth: the vendored copy of the gateway server's wire-protocol schema.
    val schemaFile = layout.projectDirectory.file("src/main/resources/sbe/gateway-schema.xml")
    systemProperty("sbe.output.dir", generatedSbeSourceDir.get().asFile.absolutePath)
    args(schemaFile.asFile.absolutePath)
    inputs.file(schemaFile)
    outputs.dir(generatedSbeSourceDir)
}

tasks.compileJava {
    dependsOn("generateSbeCodecs")
}

// The sources/javadoc jars and javadoc task consume the main source set, which now includes the
// generated SBE sources, so they must wait for codegen.
tasks.withType<Jar>().configureEach {
    dependsOn("generateSbeCodecs")
}
tasks.withType<Javadoc>().configureEach {
    dependsOn("generateSbeCodecs")
}

tasks.test {
    useJUnitPlatform()
    jvmArgs("-Dnet.bytebuddy.experimental=true")
    // Aeron/Agrona need these opens on modern JDKs.
    jvmArgs("--add-opens", "java.base/jdk.internal.misc=ALL-UNNAMED")
    jvmArgs("--add-opens", "java.base/java.util.zip=ALL-UNNAMED")
    // Forward the Aeron gateway integration-test gate (and host) from the Gradle CLI to the test JVM,
    // so `-Daeron.gateway.it=true` actually reaches the JUnit condition.
    listOf("aeron.gateway.it", "aeron.gateway.host").forEach { key ->
        System.getProperty(key)?.let { systemProperty(key, it) }
    }
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(25))
    }
    withSourcesJar()
    withJavadocJar()
}

publishing {
    repositories {
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/${System.getenv("GITHUB_REPOSITORY")}")
            credentials {
                username = System.getenv("GITHUB_ACTOR")
                password = System.getenv("GITHUB_TOKEN")
            }
        }
    }
    publications {
        create<MavenPublication>("mavenJava") {
            from(components["java"])
        }
    }
}
