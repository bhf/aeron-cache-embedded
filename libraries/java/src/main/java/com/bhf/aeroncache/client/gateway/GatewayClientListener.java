package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.gateway.messages.OperationStatus;
import com.bhf.aeroncache.gateway.messages.UpdateEventType;

import java.util.List;
import java.util.Map;

/**
 * Callback interface for responses, streamed data and updates received by a {@link GatewayClient}.
 * <p>
 * All callbacks are invoked on the {@link GatewayClient}'s agent thread (the thread driving
 * {@link GatewayClient#doWork()}), so implementations must not block.
 */
public interface GatewayClientListener {

    /**
     * A response to a request/response command (create/add/get-entry/clear/delete/remove/counter ops).
     */
    void onCommandResponse(String correlationId, OperationStatus status, String cacheId, String key, String value);

    /**
     * A batch of entries streamed in response to a getEntries command. Entries arrive in one or more
     * batches; the final batch carries {@code endOfBatch=true}. Accumulate across invocations with the
     * same {@code correlationId} until an end-of-batch frame.
     */
    void onEntries(String correlationId, OperationStatus status, String cacheId, Map<String, String> items, boolean endOfBatch);

    /**
     * A batch of cache stats streamed in response to a getStats command.
     */
    void onStats(String correlationId, OperationStatus status, List<GatewayStat> stats, boolean endOfBatch);

    /**
     * A streaming cache update pushed to a subscribed client.
     */
    void onStreamUpdate(String correlationId, UpdateEventType eventType, String cacheId, String key, String value);

    /**
     * An error correlated to a request.
     */
    void onError(String correlationId, OperationStatus status, String message);
}
