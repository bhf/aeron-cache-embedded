plugins {
    application
    java
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("com.aeron.cache:aeron-cache-embedded-client:1.0.0")
    implementation("org.slf4j:slf4j-simple:2.0.13")
}

application {
    mainClass.set("com.aeron.cache.sample.SyncSample")
}
