package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.gateway.messages.OperationStatus;
import com.bhf.aeroncache.gateway.messages.UpdateEventType;
import com.bhf.aeroncache.models.BulkOperationType;
import com.bhf.aeroncache.models.CacheOperationRequest;
import com.bhf.aeroncache.models.CacheOperationResponse;
import com.bhf.aeroncache.models.TimerInfo;
import io.aeron.Aeron;
import org.agrona.CloseHelper;
import org.agrona.concurrent.AgentRunner;
import org.agrona.concurrent.BackoffIdleStrategy;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.locks.LockSupport;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * IPC counterpart of {@link AeronGatewayClientBatchIntegrationTest}: verifies the gateway's
 * <em>batched-response reassembly</em> contract ({@code getEntries}, {@code getTimers}, {@code bulkOps}
 * responses split across multiple {@code endOfBatch}-flagged frames) over the
 * {@link TransportMedia#IPC} transport rather than UDP.
 * <p>
 * The wire encoding/decoding is identical to UDP (same SBE frames, same batch flags); this class proves
 * the batch boundaries are observable and correctly reassembled over the IPC channel too. As with the
 * UDP batch test, it drives the low-level {@link GatewayClient} directly (rather than
 * {@link AeronGatewayClient}, which hides per-batch callbacks behind its accumulating futures) and
 * asserts on the raw {@link GatewayClientListener} callbacks that:
 * <ul>
 *   <li>more than one batch frame arrives;</li>
 *   <li>exactly one frame carries {@code endOfBatch=true}, and it is the last;</li>
 *   <li>every earlier frame carries {@code endOfBatch=false};</li>
 *   <li>the union of all frames reassembles the complete, correct result set.</li>
 * </ul>
 * <p>
 * Enabled only when {@code -Daeron.gateway.ipc.it=true}. Unlike the UDP batch test (own embedded driver),
 * IPC requires this JVM to share the gateway server's media driver, so the aeron directory is resolved the
 * same way {@link AeronGatewayClientIpcIntegrationTest} resolves it: the {@code aeron.dir} system property,
 * else the {@code AERON_DIR} environment variable, else {@code "aeron"}. Enable the gateway on the monolith
 * with {@code -Daeron.gateway.enabled=true} (or {@code AERON_GATEWAY_ENABLED=true}) and
 * {@code GATEWAY_TRANSPORT_MEDIA=ipc}.
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@EnabledIfSystemProperty(named = "aeron.gateway.ipc.it", matches = "true")
class AeronGatewayClientIpcBatchIntegrationTest {

    // The gateway packs ~100 records per batch frame, so 300 entries/timers span ~3 batches — enough
    // to exercise reassembly with margin, while keeping the write load light enough for a cold CI
    // cluster (populating 1000 entries timed out on CI; see the small BULK_CHUNK below).
    private static final int ENTRY_COUNT = 300;
    /**
     * Per-bulk chunk size kept small so each populate request commits well within the client request
     * timeout even on a slow CI RAFT cluster (and under the command ring buffer's max message length).
     */
    private static final int BULK_CHUNK = 20;
    /** Value long enough that {@value #ENTRY_COUNT} entries comfortably exceed one batch frame. */
    private static final String VALUE = "v".repeat(300);
    private static final long AWAIT_SECONDS = 300;

    private Aeron aeron;
    private GatewayClient gatewayClient;
    private AgentRunner agentRunner;
    private RecordingListener listener;

    @BeforeAll
    void setUp() {
        // IPC requires this JVM to share the gateway server's media driver, so connect to the same
        // aeron.dir the server resolves (mirroring AeronGatewayClientIpcIntegrationTest) rather than
        // launching an embedded driver as the UDP batch test does.
        final String aeronDir = System.getProperty("aeron.dir",
                System.getenv().getOrDefault("AERON_DIR", "aeron"));
        aeron = Aeron.connect(new Aeron.Context().aeronDirectoryName(aeronDir));
        listener = new RecordingListener();
        gatewayClient = new GatewayClient(aeron, TransportMedia.IPC,
                null, AeronGatewayClient.DEFAULT_REQUEST_STREAM_ID,
                null, AeronGatewayClient.DEFAULT_RESPONSE_STREAM_ID,
                listener);
        agentRunner = new AgentRunner(new BackoffIdleStrategy(), Throwable::printStackTrace, null, gatewayClient);
        AgentRunner.startOnThread(agentRunner);
        assertTrue(awaitConnected(10, TimeUnit.SECONDS), "gateway did not connect over IPC");
    }

    @AfterAll
    void tearDown() {
        CloseHelper.quietClose(agentRunner);
        CloseHelper.quietClose(aeron);
    }

    @Test
    void getEntriesReassemblesAcrossMultipleBatches() throws Exception {
        final String cacheId = "it-ipc-batch-entries-" + UUID.randomUUID();
        createCache(cacheId);
        populateEntries(cacheId, ENTRY_COUNT, 0L);

        final String correlationId = "get-entries-" + UUID.randomUUID();
        final EntriesCapture capture = listener.expectEntries(correlationId);
        gatewayClient.getEntries(correlationId, cacheId);
        capture.done.get(AWAIT_SECONDS, TimeUnit.SECONDS);

        assertMultiBatch(capture.batchFlags, "getEntries");
        assertEquals(ENTRY_COUNT, capture.items.size(),
                "every entry should be reassembled from the batched response");
        for (int i = 0; i < ENTRY_COUNT; i++) {
            assertEquals(VALUE, capture.items.get("k" + i), "missing/incorrect value for k" + i);
        }

        deleteCache(cacheId);
    }

    @Test
    void getTimersReassemblesAcrossMultipleBatches() throws Exception {
        final String cacheId = "it-ipc-batch-timers-" + UUID.randomUUID();
        createCache(cacheId);
        // Each timed entry schedules a pending TTL removal timer; a large TTL keeps them all pending.
        populateEntries(cacheId, ENTRY_COUNT, 600_000L);

        final String correlationId = "get-timers-" + UUID.randomUUID();
        final TimersCapture capture = listener.expectTimers(correlationId);
        gatewayClient.getTimers(correlationId);
        capture.done.get(AWAIT_SECONDS, TimeUnit.SECONDS);

        assertMultiBatch(capture.batchFlags, "getTimers");
        // getTimers is global (all caches); assert our cache's timers all survived reassembly.
        final List<String> ours = new ArrayList<>();
        for (TimerInfo timer : capture.timers) {
            if (cacheId.equals(timer.getCacheId())) {
                assertEquals("CACHE", timer.getTimerType());
                assertTrue(timer.getDeadline() > 0, "timer deadline should be positive epoch millis");
                ours.add(timer.getKey());
            }
        }
        assertEquals(ENTRY_COUNT, ours.size(), "every pending timer for " + cacheId + " should be reassembled");

        deleteCache(cacheId);
    }

    @Test
    void bulkResponseReassemblesAcrossMultipleBatches() throws Exception {
        final String cacheId = "it-ipc-batch-bulk-" + UUID.randomUUID();
        createCache(cacheId);

        // One bulk request of GET_ITEM ops; the response carries one result per op. The bulk response
        // batches by op COUNT (~100 results per frame, regardless of value size), so 120 ops gives 2
        // batches. Executing many individual gets is slow on a cold CI cluster, so this is kept just
        // over the ~100 threshold, only populates what the bulk reads back, and leans on the very
        // generous AWAIT_SECONDS below (120 gets have been observed near ~100s on CI).
        final int bulkOpCount = 120;
        populateEntries(cacheId, bulkOpCount, 0L);
        final List<CacheOperationRequest> ops = new ArrayList<>(bulkOpCount);
        for (int i = 0; i < bulkOpCount; i++) {
            ops.add(CacheOperationRequest.builder()
                    .operationType(BulkOperationType.GET_ITEM)
                    .requestId("op-" + i)
                    .cacheId(cacheId)
                    .key("k" + i)
                    .build());
        }

        final String correlationId = "bulk-" + UUID.randomUUID();
        final BulkCapture capture = listener.expectBulk(correlationId);
        gatewayClient.bulkOps(correlationId, ops);
        capture.done.get(AWAIT_SECONDS, TimeUnit.SECONDS);

        assertMultiBatch(capture.batchFlags, "bulkOps");
        assertEquals(bulkOpCount, capture.operations.size(),
                "every bulk-operation result should be reassembled from the batched response");
        for (CacheOperationResponse response : capture.operations) {
            assertEquals(VALUE, response.getValue(), "wrong value for " + response.getRequestId());
        }

        deleteCache(cacheId);
    }

    // ------------------------------------------------------------------ helpers

    /** Asserts the batch-flag sequence: >1 frame, all but the last false, exactly the last true. */
    private static void assertMultiBatch(List<Boolean> batchFlags, String op) {
        assertTrue(batchFlags.size() >= 2,
                op + " response should span multiple batches but arrived in " + batchFlags.size()
                        + " frame(s); increase the populated count to force a split");
        for (int i = 0; i < batchFlags.size() - 1; i++) {
            assertFalse(batchFlags.get(i), op + " non-final batch " + i + " must carry endOfBatch=false");
        }
        assertTrue(batchFlags.get(batchFlags.size() - 1), op + " final batch must carry endOfBatch=true");
    }

    private void createCache(String cacheId) throws Exception {
        final String correlationId = "create-" + UUID.randomUUID();
        final CompletableFuture<OperationStatus> done = listener.expectCommand(correlationId);
        gatewayClient.createCache(correlationId, cacheId);
        done.get(AWAIT_SECONDS, TimeUnit.SECONDS);
    }

    private void deleteCache(String cacheId) throws Exception {
        final String correlationId = "delete-" + UUID.randomUUID();
        final CompletableFuture<OperationStatus> done = listener.expectCommand(correlationId);
        gatewayClient.deleteCache(correlationId, cacheId);
        done.get(AWAIT_SECONDS, TimeUnit.SECONDS);
    }

    /** Inserts {@code count} entries (keys {@code k0..k(count-1)}) via chunked bulk ADD_ITEM. */
    private void populateEntries(String cacheId, int count, long ttl) throws Exception {
        for (int start = 0; start < count; start += BULK_CHUNK) {
            final int end = Math.min(start + BULK_CHUNK, count);
            final List<CacheOperationRequest> ops = new ArrayList<>(end - start);
            for (int i = start; i < end; i++) {
                final CacheOperationRequest.Builder op = CacheOperationRequest.builder()
                        .operationType(BulkOperationType.ADD_ITEM)
                        .requestId("add-" + i)
                        .cacheId(cacheId)
                        .key("k" + i)
                        .value(VALUE);
                if (ttl > 0) {
                    op.ttl(ttl);
                }
                ops.add(op.build());
            }
            final String correlationId = "populate-" + UUID.randomUUID();
            final BulkCapture capture = listener.expectBulk(correlationId);
            gatewayClient.bulkOps(correlationId, ops);
            capture.done.get(AWAIT_SECONDS, TimeUnit.SECONDS);
        }
    }

    private boolean awaitConnected(long timeout, TimeUnit unit) {
        final long deadline = System.nanoTime() + unit.toNanos(timeout);
        while (System.nanoTime() < deadline) {
            if (gatewayClient.isConnected()) {
                return true;
            }
            gatewayClient.getStats("connection-warmup");
            LockSupport.parkNanos(TimeUnit.MILLISECONDS.toNanos(250));
        }
        return gatewayClient.isConnected();
    }

    // ------------------------------------------------------------------ recording listener

    private static final class EntriesCapture {
        final List<Boolean> batchFlags = new CopyOnWriteArrayList<>();
        final Map<String, String> items = new ConcurrentHashMap<>();
        final CompletableFuture<Void> done = new CompletableFuture<>();
    }

    private static final class TimersCapture {
        final List<Boolean> batchFlags = new CopyOnWriteArrayList<>();
        final List<TimerInfo> timers = new CopyOnWriteArrayList<>();
        final CompletableFuture<Void> done = new CompletableFuture<>();
    }

    private static final class BulkCapture {
        final List<Boolean> batchFlags = new CopyOnWriteArrayList<>();
        final List<CacheOperationResponse> operations = new CopyOnWriteArrayList<>();
        final CompletableFuture<Void> done = new CompletableFuture<>();
    }

    /**
     * Records the raw batch callbacks per correlationId. Callbacks fire on the {@link GatewayClient}
     * agent thread; captures must be registered (via {@code expect*}) before the command is sent.
     */
    private static final class RecordingListener implements GatewayClientListener {
        private final Map<String, CompletableFuture<OperationStatus>> commands = new ConcurrentHashMap<>();
        private final Map<String, EntriesCapture> entries = new ConcurrentHashMap<>();
        private final Map<String, TimersCapture> timers = new ConcurrentHashMap<>();
        private final Map<String, BulkCapture> bulk = new ConcurrentHashMap<>();

        CompletableFuture<OperationStatus> expectCommand(String correlationId) {
            final CompletableFuture<OperationStatus> future = new CompletableFuture<>();
            commands.put(correlationId, future);
            return future;
        }

        EntriesCapture expectEntries(String correlationId) {
            final EntriesCapture capture = new EntriesCapture();
            entries.put(correlationId, capture);
            return capture;
        }

        TimersCapture expectTimers(String correlationId) {
            final TimersCapture capture = new TimersCapture();
            timers.put(correlationId, capture);
            return capture;
        }

        BulkCapture expectBulk(String correlationId) {
            final BulkCapture capture = new BulkCapture();
            bulk.put(correlationId, capture);
            return capture;
        }

        @Override
        public void onCommandResponse(String correlationId, OperationStatus status, String cacheId, String key, String value) {
            final CompletableFuture<OperationStatus> future = commands.remove(correlationId);
            if (future != null) {
                future.complete(status);
            }
        }

        @Override
        public void onEntries(String correlationId, OperationStatus status, String cacheId, Map<String, String> items, boolean endOfBatch) {
            final EntriesCapture capture = entries.get(correlationId);
            if (capture == null) {
                return;
            }
            capture.batchFlags.add(endOfBatch);
            capture.items.putAll(items);
            if (endOfBatch) {
                entries.remove(correlationId);
                capture.done.complete(null);
            }
        }

        @Override
        public void onStats(String correlationId, OperationStatus status, List<GatewayStat> stats, boolean endOfBatch) {
            // Warmup probes land here and are ignored.
        }

        @Override
        public void onTimers(String correlationId, OperationStatus status, List<TimerInfo> timerBatch, boolean endOfBatch) {
            final TimersCapture capture = timers.get(correlationId);
            if (capture == null) {
                return;
            }
            capture.batchFlags.add(endOfBatch);
            capture.timers.addAll(timerBatch);
            if (endOfBatch) {
                timers.remove(correlationId);
                capture.done.complete(null);
            }
        }

        @Override
        public void onBulkResponse(String correlationId, List<CacheOperationResponse> operations, boolean endOfBatch) {
            final BulkCapture capture = bulk.get(correlationId);
            if (capture == null) {
                return;
            }
            capture.batchFlags.add(endOfBatch);
            capture.operations.addAll(operations);
            if (endOfBatch) {
                bulk.remove(correlationId);
                capture.done.complete(null);
            }
        }

        @Override
        public void onStreamUpdate(String correlationId, UpdateEventType eventType, String cacheId, String key, String value) {
            // Not exercised by these tests.
        }

        @Override
        public void onError(String correlationId, OperationStatus status, String message) {
            final RuntimeException error = new RuntimeException("gateway error " + status + ": " + message);
            final CompletableFuture<OperationStatus> command = commands.remove(correlationId);
            if (command != null) {
                command.completeExceptionally(error);
            }
            final EntriesCapture entriesCapture = entries.remove(correlationId);
            if (entriesCapture != null) {
                entriesCapture.done.completeExceptionally(error);
            }
            final TimersCapture timersCapture = timers.remove(correlationId);
            if (timersCapture != null) {
                timersCapture.done.completeExceptionally(error);
            }
            final BulkCapture bulkCapture = bulk.remove(correlationId);
            if (bulkCapture != null) {
                bulkCapture.done.completeExceptionally(error);
            }
        }
    }
}
