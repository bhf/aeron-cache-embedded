package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.gateway.messages.OperationStatus;
import com.bhf.aeroncache.gateway.messages.UpdateEventType;
import com.bhf.aeroncache.models.BulkOperationType;
import com.bhf.aeroncache.models.CacheOperationRequest;
import com.bhf.aeroncache.models.CacheOperationResponse;
import com.bhf.aeroncache.models.TimerInfo;
import io.aeron.Aeron;
import io.aeron.driver.MediaDriver;
import io.aeron.driver.ThreadingMode;
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
 * Verifies the gateway's <em>batched-response reassembly</em> contract: {@code getEntries},
 * {@code getTimers} and {@code bulkOps} responses are streamed as one or more batch frames, each
 * carrying an {@code endOfBatch} flag, and the client must accumulate frames with the same
 * {@code correlationId} until the final frame ({@code endOfBatch=true}).
 * <p>
 * The other gateway integration tests insert a single item, so their responses fit in one batch and
 * never exercise reassembly. These tests deliberately populate <em>enough</em> entries/timers to force
 * the server to split the response across multiple batches, then assert on the raw
 * {@link GatewayClientListener} callbacks that:
 * <ul>
 *   <li>more than one batch frame arrives;</li>
 *   <li>exactly one frame carries {@code endOfBatch=true}, and it is the last;</li>
 *   <li>every earlier frame carries {@code endOfBatch=false};</li>
 *   <li>the union of all frames reassembles the complete, correct result set.</li>
 * </ul>
 * This drives the low-level {@link GatewayClient} directly (rather than {@link AeronGatewayClient},
 * which hides the per-batch callbacks behind its accumulating futures) so the batch boundaries are
 * observable.
 * <p>
 * Enabled only when {@code -Daeron.gateway.it=true}; see {@link AeronGatewayClientIntegrationTest} for
 * the gateway/backend prerequisites. Launches its own embedded media driver and talks to the gateway
 * over UDP.
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@EnabledIfSystemProperty(named = "aeron.gateway.it", matches = "true")
class AeronGatewayClientBatchIntegrationTest {

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
    private static final long AWAIT_SECONDS = 60;

    private MediaDriver mediaDriver;
    private Aeron aeron;
    private GatewayClient gatewayClient;
    private AgentRunner agentRunner;
    private RecordingListener listener;

    @BeforeAll
    void setUp() {
        final String host = System.getProperty("aeron.gateway.host", "127.0.0.1");
        mediaDriver = MediaDriver.launchEmbedded(new MediaDriver.Context()
                .threadingMode(ThreadingMode.SHARED)
                .dirDeleteOnStart(true)
                .dirDeleteOnShutdown(true));
        aeron = Aeron.connect(new Aeron.Context().aeronDirectoryName(mediaDriver.aeronDirectoryName()));
        listener = new RecordingListener();
        gatewayClient = new GatewayClient(aeron, TransportMedia.UDP,
                host + ":" + AeronGatewayClient.DEFAULT_REQUEST_PORT, AeronGatewayClient.DEFAULT_REQUEST_STREAM_ID,
                host + ":" + AeronGatewayClient.DEFAULT_RESPONSE_CONTROL_PORT, AeronGatewayClient.DEFAULT_RESPONSE_STREAM_ID,
                listener);
        agentRunner = new AgentRunner(new BackoffIdleStrategy(), Throwable::printStackTrace, null, gatewayClient);
        AgentRunner.startOnThread(agentRunner);
        assertTrue(awaitConnected(10, TimeUnit.SECONDS), "gateway did not connect over UDP");
    }

    @AfterAll
    void tearDown() {
        CloseHelper.quietClose(agentRunner);
        CloseHelper.quietClose(aeron);
        CloseHelper.quietClose(mediaDriver);
    }

    @Test
    void getEntriesReassemblesAcrossMultipleBatches() throws Exception {
        final String cacheId = "it-batch-entries-" + UUID.randomUUID();
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
        final String cacheId = "it-batch-timers-" + UUID.randomUUID();
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
        final String cacheId = "it-batch-bulk-" + UUID.randomUUID();
        createCache(cacheId);
        populateEntries(cacheId, ENTRY_COUNT, 0L);

        // One bulk request of GET_ITEM ops; the response carries one result per op and, with this many
        // 300-char values, spans multiple batch frames. Kept under the ring buffer's max message length.
        final int bulkOpCount = 300;
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
