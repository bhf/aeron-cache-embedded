package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.models.CacheUpdateEvent;
import com.bhf.aeroncache.models.CreateResponse;
import io.aeron.Aeron;
import io.aeron.driver.MediaDriver;
import io.aeron.driver.ThreadingMode;
import org.agrona.CloseHelper;
import org.agrona.concurrent.AgentRunner;
import org.agrona.concurrent.SleepingMillisIdleStrategy;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.TimeUnit;
import java.util.function.BooleanSupplier;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Proves {@link AeronGatewayClient} transparently reconnects when the gateway drops and returns, and that
 * active subscriptions are replayed on reconnect, using an in-process {@link LoopbackGatewayServer} over
 * IPC. The gateway "drop" is simulated by stopping the server's agent (closing its request subscription
 * and per-session response publications); "recovery" by starting a fresh server on the same streams. No
 * network and no real backend.
 */
class AeronGatewayClientReconnectLoopbackTest {

    private static final int REQUEST_STREAM_ID = 2100;
    private static final int RESPONSE_STREAM_ID = 2101;
    private static final long TIMEOUT_MS = 20_000;

    private static MediaDriver mediaDriver;
    private static Aeron aeron;

    @BeforeAll
    static void startEnvironment() {
        mediaDriver = MediaDriver.launchEmbedded(new MediaDriver.Context()
                .threadingMode(ThreadingMode.SHARED)
                .dirDeleteOnStart(true)
                .dirDeleteOnShutdown(true));
        aeron = Aeron.connect(new Aeron.Context().aeronDirectoryName(mediaDriver.aeronDirectoryName()));
    }

    @AfterAll
    static void stopEnvironment() {
        CloseHelper.quietClose(aeron);
        CloseHelper.quietClose(mediaDriver);
    }

    private record ServerHandle(AgentRunner runner, LoopbackGatewayServer server) {
    }

    private static ServerHandle startServer() {
        final LoopbackGatewayServer server = new LoopbackGatewayServer(
                aeron, TransportMedia.IPC, null, null, REQUEST_STREAM_ID, RESPONSE_STREAM_ID);
        final AgentRunner runner =
                new AgentRunner(new SleepingMillisIdleStrategy(1), Throwable::printStackTrace, null, server);
        AgentRunner.startOnThread(runner);
        return new ServerHandle(runner, server);
    }

    @Test
    void reconnectsAndReplaysSubscriptionsAfterGatewayRestart() throws Exception {
        ServerHandle gateway = startServer();
        // Endpoints are ignored for IPC; the client shares the one media driver with the server.
        final AeronGatewayClient client = new AeronGatewayClient(
                aeron, TransportMedia.IPC, null, REQUEST_STREAM_ID, null, RESPONSE_STREAM_ID);
        try {
            assertTrue(client.awaitConnected(TIMEOUT_MS, TimeUnit.MILLISECONDS), "did not connect initially");

            final CreateResponse before = client.createCacheAsync("before").get(TIMEOUT_MS, TimeUnit.MILLISECONDS);
            assertEquals("SUCCESS", before.getOperationStatus());

            // Subscribe and prove the subscription delivers before any drop.
            final List<CacheUpdateEvent> events = new CopyOnWriteArrayList<>();
            client.subscribe("sub-cache", events::add);
            final LoopbackGatewayServer server1 = gateway.server;
            awaitTrue(() -> server1.isSubscribed("sub-cache"), "server never saw the subscription");
            assertTrue(produceUntilSeen(server1, events, "baseline"), "baseline update not delivered");

            // Drop the gateway: stopping the server closes its subscription and response publications,
            // so the client's publication/subscription go unconnected.
            CloseHelper.quietClose(gateway.runner);
            awaitTrue(() -> !client.isConnected(), "client did not observe the gateway drop");
            // Keep the gateway down long enough for the connection monitor (250ms cadence) to observe
            // the drop and begin reconnecting (fail-pending + rebuild), so the restart below exercises
            // the full reconnect+replay path rather than Aeron silently re-forming the original images.
            Thread.sleep(1_000);

            // Bring the gateway back on the same streams. The client's connection monitor rebuilds its
            // publication/subscription, warms up the response channel, and replays active subscriptions.
            gateway = startServer();
            final LoopbackGatewayServer server2 = gateway.server;
            awaitTrue(client::isConnected, "client did not reconnect after the gateway returned");

            // Commands work again post-reconnect.
            final CreateResponse after = client.createCacheAsync("after").get(TIMEOUT_MS, TimeUnit.MILLISECONDS);
            assertEquals("SUCCESS", after.getOperationStatus());

            // The replayed subscribe must reach the fresh gateway (its subscribed set started empty)...
            awaitTrue(() -> server2.isSubscribed("sub-cache"),
                    "subscription was not replayed to the gateway after reconnect");
            // ...and the subscription must resume delivering updates.
            assertTrue(produceUntilSeen(server2, events, "postreconnect"),
                    "subscription did not resume after reconnect (replay failed)");
        } finally {
            client.close();
            CloseHelper.quietClose(gateway.runner);
        }
    }

    /**
     * Produce a fresh-keyed stream update each round until the client's subscription delivers it (or the
     * timeout elapses), so the assertion tolerates the reconnect gap-window and any registration lag.
     */
    private static boolean produceUntilSeen(LoopbackGatewayServer server, List<CacheUpdateEvent> events,
                                            String keyPrefix) throws InterruptedException {
        final long deadline = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(TIMEOUT_MS);
        int i = 0;
        while (System.nanoTime() < deadline) {
            final String key = keyPrefix + "-" + i++;
            server.publishStreamUpdate("sub-cache", key, "v");
            final long roundDeadline = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(500);
            while (System.nanoTime() < roundDeadline) {
                if (events.stream().anyMatch(e -> key.equals(e.getItemKey()))) {
                    return true;
                }
                Thread.sleep(20);
            }
        }
        return false;
    }

    private static void awaitTrue(BooleanSupplier condition, String message) throws InterruptedException {
        final long deadline = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(TIMEOUT_MS);
        while (System.nanoTime() < deadline) {
            if (condition.getAsBoolean()) {
                return;
            }
            Thread.sleep(10);
        }
        assertTrue(condition.getAsBoolean(), message);
    }
}
