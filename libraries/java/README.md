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

## Counters

Counter caches hold `int64` values and add `increment`, `decrement`, and `set` operations:

```java
import com.aeron.cache.client.AeronCacheClient;
import com.aeron.cache.client.EmbeddedCounterCache;

AeronCacheClient client = new AeronCacheClient("http://localhost:7070", "ws://localhost:7071");
client.createCounterCache("counter-cache");
EmbeddedCounterCache counters = client.getCounterCache("counter-cache");

counters.put("requests", 10);
counters.increment("requests", 5); // -> 15
counters.decrement("requests", 3); // -> 12
counters.set("requests", 100);     // -> 100
System.out.println(counters.get("requests").getValue());
```

## Bulk Operations

```java
import com.aeron.cache.models.*;
import java.util.List;

CacheOperationRequest op = new CacheOperationRequest();
op.setOperationType(BulkOperationType.INCREMENT_COUNTER);
op.setCacheId("counter-cache");
op.setKey("requests");
op.setCounterValue(5L);

BulkCacheOpsResponse result = client.bulkOps(new BulkCacheOpsRequest("req-1", List.of(op)));
```
