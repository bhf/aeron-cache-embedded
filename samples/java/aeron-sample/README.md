# Aeron Gateway Sample (Java)

Demonstrates the **Aeron transport** for Aeron Cache using
[`AeronGatewayClient`](../../../libraries/java/src/main/java/com/bhf/aeroncache/client/gateway/AeronGatewayClient.java) —
the low-latency, bidirectional alternative to the HTTP+WS client. It covers:

- cache CRUD and a streaming subscription,
- an **embedded cache** (`EmbeddedAeronCache`) whose local map is kept in sync from streaming updates,
  so writes go to the gateway while reads are served locally with no network round-trip,
- counter operations,

all over a single Aeron connection.

## Prerequisites

The backend must have the Aeron gateway enabled:

```
-Daeron.gateway.enabled=true
```

(or the environment variable `AERON_GATEWAY_ENABLED=true`). The gateway binds the request
endpoint on port `7075` and the response control endpoint on port `7076` by default.

## Running

By default the sample connects over **UDP**, launching its own embedded media driver, so no separate
Aeron media driver is needed.

```bash
./gradlew run
```

Point it at a non-local gateway with:

```bash
./gradlew run -Daeron.gateway.host=<host>
```

### IPC transport media

The gateway also supports **IPC** for co-located clients — lower latency, no network stack, but the
client and gateway server must share the same media driver (same host, same `aeron.dir`). Start the
backend with the shared directory and `GATEWAY_TRANSPORT_MEDIA=ipc`:

```bash
AERON_DIR=/tmp/aeron-cache-shared GATEWAY_TRANSPORT_MEDIA=ipc aeron-cache
```

Then run the sample against the same directory, over IPC:

```bash
./gradlew run -Daeron.gateway.media=ipc -Daeron.dir=/tmp/aeron-cache-shared
```

For IPC the sample does **not** launch an embedded driver — `-Daeron.dir` (or the `AERON_DIR`
environment variable) must point at the driver the gateway server is using, and
`-Daeron.gateway.host` is ignored (IPC has no network endpoints).

## Container image

The sample is also published as a container image, so it can run as a client/writer against a remote
gateway with no build — for example as an in-cluster exerciser in a Kubernetes deployment:

```
ghcr.io/bhf/aeroncache-embedded-aeron-sample
```

It is built with [Jib](https://github.com/GoogleContainerTools/jib) (no Dockerfile, no Docker daemon)
from `build.gradle.kts`, and published by the `release-aeron-sample.yml` workflow on an
`aeron-sample-v*` tag or manual dispatch. Build it locally with `./gradlew jib` (push to a registry)
or `./gradlew jibDockerBuild` (load into the local Docker daemon).

Point it at a gateway at **runtime** via the JVM's standard `JAVA_TOOL_OPTIONS` — the image bakes the
`--add-opens` flags and main class into its entrypoint, so pass only your `-D` overrides:

```bash
docker run --rm \
  -e JAVA_TOOL_OPTIONS="-Daeron.gateway.host=my-gateway-host" \
  ghcr.io/bhf/aeroncache-embedded-aeron-sample
```

The image exposes no ports: the client only dials out to the gateway (request `:7075`,
response-control `:7076`) — the gateway creates each client's response publication lazily over an MDC
control channel — so no inbound listener is needed. The embedded UDP media driver writes to a
temporary Aeron directory, so give the container enough shared memory (e.g. a larger `/dev/shm`) if
you hit media-driver space errors.
