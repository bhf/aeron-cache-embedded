plugins {
    `java-library`
    `maven-publish`
}

group = "com.aeron.cache"
version = "1.0.0"

repositories {
    mavenCentral()
}

dependencies {
    // No external dependencies required as it uses Java 11 HTTP Client
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(21))
    }
    withSourcesJar()
    withJavadocJar()
}

publishing {
    publications {
        create<MavenPublication>("mavenJava") {
            from(components["java"])
        }
    }
}
