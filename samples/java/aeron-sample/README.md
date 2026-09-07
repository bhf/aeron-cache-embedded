# Aeron Gateway Sample (Java)

Demonstrates the **Aeron transport** for Aeron Cache using
[`AeronGatewayClient`](../../../libraries/java/src/main/java/com/bhf/aeroncache/client/gateway/AeronGatewayClient.java) —
the low-latency, bidirectional alternative to the HTTP+WS client. It covers cache CRUD, a streaming
subscription, and counter operations, all over a single Aeron connection.

## Prerequisites

The backend must have the Aeron gateway enabled:

```
-Daeron.transport.gateway.enabled=true
```

(or the environment variable `AERON_TRANSPORT_GATEWAY_ENABLED=true`). The gateway binds the request
endpoint on port `7075` and the response control endpoint on port `7076` by default.

## Running

The sample launches its own embedded media driver, so no separate Aeron media driver is needed.

```bash
./gradlew run
```

Point it at a non-local gateway with:

```bash
./gradlew run -Daeron.gateway.host=<host>
```
