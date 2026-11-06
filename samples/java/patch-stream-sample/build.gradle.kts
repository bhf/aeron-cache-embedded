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
    // EmbeddedObjectCache.getLocal returns a Jackson ObjectNode; needed on the sample classpath.
    implementation("com.fasterxml.jackson.core:jackson-databind:2.16.1")
}

application {
    mainClass.set("com.aeron.cache.sample.PatchStreamSample")
}
