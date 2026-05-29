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
    mainClass.set("com.aeron.cache.sample.SyncSample")
}
