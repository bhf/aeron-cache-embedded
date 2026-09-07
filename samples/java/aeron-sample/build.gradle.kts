plugins {
    application
    java
}

repositories {
    mavenLocal()
    mavenCentral()
}

dependencies {
    implementation("com.bhf.aeroncache:aeron-cache-embedded-client:1.0.1")
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
