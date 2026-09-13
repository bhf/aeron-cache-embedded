package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.gateway.messages.BooleanType;
import com.bhf.aeroncache.gateway.messages.BulkOperationType;
import com.bhf.aeroncache.gateway.messages.GatewayBulkRequestEncoder;
import com.bhf.aeroncache.gateway.messages.GatewayCommandEncoder;
import com.bhf.aeroncache.gateway.messages.GatewaySubscribeEncoder;
import com.bhf.aeroncache.gateway.messages.GatewayUnsubscribeEncoder;
import com.bhf.aeroncache.gateway.messages.MessageHeaderEncoder;
import com.bhf.aeroncache.gateway.messages.SubscriptionMode;
import com.bhf.aeroncache.models.CacheOperationRequest;
import org.agrona.MutableDirectBuffer;

import java.util.List;

/**
 * Encodes gateway request frames (command / subscribe / unsubscribe) into a buffer.
 * <p>
 * Not thread safe: a single instance reuses its encoders, so access must be externally serialised.
 * {@link GatewayClient} holds one instance per calling thread (thread-local) so each writer is only
 * ever touched by a single thread.
 */
class GatewayRequestWriter {

    private final MessageHeaderEncoder headerEncoder = new MessageHeaderEncoder();
    private final GatewayCommandEncoder commandEncoder = new GatewayCommandEncoder();
    private final GatewaySubscribeEncoder subscribeEncoder = new GatewaySubscribeEncoder();
    private final GatewayUnsubscribeEncoder unsubscribeEncoder = new GatewayUnsubscribeEncoder();
    private final GatewayBulkRequestEncoder bulkRequestEncoder = new GatewayBulkRequestEncoder();

    /**
     * Encode a command frame.
     *
     * @return the encoded length in bytes.
     */
    int encodeCommand(MutableDirectBuffer buffer, int msgType, long ttl, long counterValue,
                      String correlationId, String cacheId, String key, String value) {
        commandEncoder.wrapAndApplyHeader(buffer, 0, headerEncoder)
                .msgType(msgType)
                .ttl(ttl)
                .counterValue(counterValue)
                .correlationId(nullSafe(correlationId))
                .cacheId(nullSafe(cacheId))
                .key(nullSafe(key))
                .value(nullSafe(value));
        return commandEncoder.limit();
    }

    /**
     * Encode a subscribe frame with the default {@link SubscriptionMode#FULL} mode and no per-cache key
     * filter.
     *
     * @return the encoded length in bytes.
     */
    int encodeSubscribe(MutableDirectBuffer buffer, String correlationId, List<String> cacheIds,
                        boolean sendSnapshot, boolean counters) {
        return encodeSubscribe(buffer, correlationId, cacheIds, sendSnapshot, counters, SubscriptionMode.FULL, "");
    }

    /**
     * Encode a subscribe frame with an explicit subscription {@code mode} and optional per-cache
     * {@code key} filter. The {@code mode} and {@code key} apply to every requested cache id.
     *
     * @return the encoded length in bytes.
     */
    int encodeSubscribe(MutableDirectBuffer buffer, String correlationId, List<String> cacheIds,
                        boolean sendSnapshot, boolean counters, SubscriptionMode mode, String key) {
        final SubscriptionMode subscriptionMode = mode == null ? SubscriptionMode.FULL : mode;
        var subscribeEnc = subscribeEncoder.wrapAndApplyHeader(buffer, 0, headerEncoder)
                .sendSnapshot(mapBoolean(sendSnapshot))
                .counters(mapBoolean(counters));
        var group = subscribeEnc.cacheIdsCount(cacheIds.size());
        for (String cacheId : cacheIds) {
            group.next()
                    .mode(subscriptionMode)
                    .cacheId(nullSafe(cacheId))
                    .key(nullSafe(key));
        }
        subscribeEnc.correlationId(nullSafe(correlationId));
        return subscribeEncoder.limit();
    }

    /**
     * Encode a bulk-operations request frame. Each {@link CacheOperationRequest} is written as one entry
     * in the {@code operations} group; the HTTP {@link com.bhf.aeroncache.models.BulkOperationType} is
     * mapped onto the SBE {@link BulkOperationType} by name.
     *
     * @return the encoded length in bytes.
     */
    int encodeBulkRequest(MutableDirectBuffer buffer, String correlationId, List<CacheOperationRequest> ops) {
        var bulkEnc = bulkRequestEncoder.wrapAndApplyHeader(buffer, 0, headerEncoder);
        var group = bulkEnc.operationsCount(ops.size());
        for (CacheOperationRequest op : ops) {
            group.next()
                    .operationType(mapOperationType(op.getOperationType()))
                    .ttl(op.getTtl() == null ? 0L : op.getTtl())
                    .counterValue(op.getCounterValue() == null ? 0L : op.getCounterValue())
                    .requestId(nullSafe(op.getRequestId()))
                    .cacheId(nullSafe(op.getCacheId()))
                    .key(nullSafe(op.getKey()))
                    .value(nullSafe(op.getValue()));
        }
        bulkEnc.correlationId(nullSafe(correlationId));
        return bulkRequestEncoder.limit();
    }

    /**
     * Encode an unsubscribe frame.
     *
     * @return the encoded length in bytes.
     */
    int encodeUnsubscribe(MutableDirectBuffer buffer, String correlationId, String cacheId, boolean counters) {
        unsubscribeEncoder.wrapAndApplyHeader(buffer, 0, headerEncoder)
                .counters(mapBoolean(counters))
                .correlationId(nullSafe(correlationId))
                .cacheId(nullSafe(cacheId));
        return unsubscribeEncoder.limit();
    }

    private static BooleanType mapBoolean(boolean value) {
        return value ? BooleanType.T : BooleanType.F;
    }

    private static BulkOperationType mapOperationType(com.bhf.aeroncache.models.BulkOperationType type) {
        return type == null ? BulkOperationType.NONE : BulkOperationType.valueOf(type.name());
    }

    private static String nullSafe(String value) {
        return value == null ? "" : value;
    }
}
