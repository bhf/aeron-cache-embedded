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

Counter caches hold `int64` values and add `increment`, `decrement`, `set`, and timed-put operations:

```java
import com.bhf.aeroncache.client.AeronCacheClient;
import com.bhf.aeroncache.client.EmbeddedCounterCache;

AeronCacheClient client = new AeronCacheClient("http://localhost:7070", "ws://localhost:7071");
client.createCounterCache("counter-cache");
EmbeddedCounterCache counters = client.getCounterCache("counter-cache");

counters.put("requests", 10);
counters.increment("requests", 5); // -> 15
counters.decrement("requests", 3); // -> 12
counters.set("requests", 100);     // -> 100
System.out.println(counters.get("requests").getValue());
```

Counter operations are also available via `bulkOps` using the counter `BulkOperationType` values and the `counterValue` field on `CacheOperationRequest`.

## Transports: HTTP+WS or Aeron

The library offers two transports for the same operations. Pick whichever suits your deployment — you can even use both:

- **HTTP + WebSocket** — [`AeronCacheClient`](src/main/java/com/bhf/aeroncache/client/AeronCacheClient.java): REST for commands, WebSocket for streaming updates. No media driver required.
- **Aeron gateway** — [`AeronGatewayClient`](src/main/java/com/bhf/aeroncache/client/gateway/AeronGatewayClient.java): a single low-latency, bidirectional Aeron connection carrying both commands and streaming updates, using the shared SBE wire protocol (`gateway-schema.xml`).

Enable the Aeron gateway on the backend with `-Daeron.transport.gateway.enabled=true` (or `AERON_TRANSPORT_GATEWAY_ENABLED=true`). By default it binds the request endpoint on port `7075` (stream `100`) and the response control endpoint on port `7076` (stream `101`).

### Aeron usage

`AeronGatewayClient` mirrors the HTTP client's method names and response models, but is asynchronous: each method has a `CompletableFuture` variant, and the synchronous variant simply blocks on it.

```java
import com.bhf.aeroncache.client.gateway.AeronGatewayClient;
import com.bhf.aeroncache.client.gateway.GatewaySubscription;
import io.aeron.driver.MediaDriver;

import java.util.concurrent.TimeUnit;

// An embedded media driver keeps the example self-contained; the gateway is reached over UDP.
try (MediaDriver driver = MediaDriver.launchEmbedded();
     AeronGatewayClient client = AeronGatewayClient.connect(driver.aeronDirectoryName(), "127.0.0.1")) {

    client.awaitConnected(10, TimeUnit.SECONDS);

    client.createCache("my-cache");
    client.putItem("my-cache", "key", "value");
    System.out.println(client.getItem("my-cache", "key").getValue());

    // Streaming updates over the same connection; closing the handle unsubscribes.
    try (GatewaySubscription sub = client.subscribe("my-cache",
            e -> System.out.println(e.getEventType() + " " + e.getItemKey() + "=" + e.getItemValue()))) {
        client.putItem("my-cache", "streamed", "value");
        Thread.sleep(1000);
    }

    // Counters ride the same transport.
    client.createCounterCache("counters");
    client.incrementCounter("counters", "hits", 5);
}
```

The Aeron transport also exposes operations not available over HTTP+WS: `getCacheItems` (full snapshot), `getStats`, and `clearCache`.

### Transport-neutral embedded caches

Both clients implement the [`CacheTransport`](src/main/java/com/bhf/aeroncache/client/CacheTransport.java) interface, so the local-mirroring `EmbeddedAeronCache`/`EmbeddedCounterCache` work identically over either transport. Use the transport-neutral `subscribe(Consumer<...>)` (it returns an `AutoCloseable` that unsubscribes on close):

```java
// `client` may be an AeronCacheClient (HTTP+WS) or an AeronGatewayClient (Aeron)
EmbeddedAeronCache cache = client.getCache("my-cache");
try (AutoCloseable sub = cache.subscribe(event -> System.out.println("update: " + event.getItemKey()))) {
    cache.put("key", "value");
    // ... the local mirror is kept up to date from streaming updates
    System.out.println(cache.getLocal("key"));
}
```

The WebSocket-listener form `subscribe(AeronCacheSubscriber)` remains available on the HTTP transport for backwards compatibility.

> **Note:** The SBE codecs are generated at build time from the vendored `src/main/resources/sbe/gateway-schema.xml` via the `generateSbeCodecs` Gradle task. Aeron/Agrona require `--add-opens java.base/jdk.internal.misc=ALL-UNNAMED --add-opens java.base/java.util.zip=ALL-UNNAMED` on the JVM running your application.
