# Aeron Cache Java Client

This is the Java client library for Aeron Cache.

## Installation

Add the following to your `build.gradle.kts` dependencies:

```kotlin
implementation("com.aeron.cache:aeron-cache-embedded-client:1.0.0")
```

## Usage

```java
import com.aeron.cache.client.AeronCacheClient;
import com.aeron.cache.client.EmbeddedAeronCache;

public class Example {
    public static void main(String[] args) throws Exception {
        AeronCacheClient client = new AeronCacheClient("http://localhost:7070", "ws://localhost:7071");
        EmbeddedAeronCache cache = client.getCache("my-cache");

        // Remote writes
        cache.put("key", "value");

        // Local reads (from embedded cache)
        System.out.println(cache.getLocal("key"));
    }
}
```
