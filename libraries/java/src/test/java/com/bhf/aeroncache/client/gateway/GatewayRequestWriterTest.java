package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.gateway.messages.BooleanType;
import com.bhf.aeroncache.gateway.messages.GatewayBulkRequestDecoder;
import com.bhf.aeroncache.gateway.messages.GatewayBulkResponseDecoder;
import com.bhf.aeroncache.gateway.messages.GatewayBulkResponseEncoder;
import com.bhf.aeroncache.gateway.messages.GatewayCommandDecoder;
import com.bhf.aeroncache.gateway.messages.GatewaySubscribeAckDecoder;
import com.bhf.aeroncache.gateway.messages.GatewaySubscribeAckEncoder;
import com.bhf.aeroncache.gateway.messages.GatewaySubscribeDecoder;
import com.bhf.aeroncache.gateway.messages.GatewayUnsubscribeDecoder;
import com.bhf.aeroncache.gateway.messages.MessageHeaderDecoder;
import com.bhf.aeroncache.gateway.messages.MessageHeaderEncoder;
import com.bhf.aeroncache.gateway.messages.OperationStatus;
import com.bhf.aeroncache.gateway.messages.SubscriptionMode;
import com.bhf.aeroncache.models.BulkOperationType;
import com.bhf.aeroncache.models.CacheOperationRequest;
import org.agrona.ExpandableArrayBuffer;
import org.agrona.MutableDirectBuffer;
import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Verifies the SBE encoders round-trip cleanly against the generated decoders, so the wire contract
 * matches the vendored {@code gateway-schema.xml}.
 */
class GatewayRequestWriterTest {

    private final GatewayRequestWriter writer = new GatewayRequestWriter();
    private final MutableDirectBuffer buffer = new ExpandableArrayBuffer(4096);
    private final MessageHeaderDecoder headerDecoder = new MessageHeaderDecoder();

    @Test
    void encodesCommandFrame() {
        writer.encodeCommand(buffer, CacheRequestMessageTypes.ADD_COUNTER_ENTRY_MSG_ID, 1234L, 42L,
                "corr-1", "cacheA", "keyA", "valueA");

        headerDecoder.wrap(buffer, 0);
        assertEquals(GatewayCommandDecoder.TEMPLATE_ID, headerDecoder.templateId());

        final GatewayCommandDecoder decoder = new GatewayCommandDecoder();
        decoder.wrap(buffer, headerDecoder.encodedLength(), headerDecoder.blockLength(), headerDecoder.version());
        assertEquals(CacheRequestMessageTypes.ADD_COUNTER_ENTRY_MSG_ID, decoder.msgType());
        assertEquals(1234L, decoder.ttl());
        assertEquals(42L, decoder.counterValue());
        assertEquals("corr-1", decoder.correlationId());
        assertEquals("cacheA", decoder.cacheId());
        assertEquals("keyA", decoder.key());
        assertEquals("valueA", decoder.value());
    }

    @Test
    void encodesCommandFrameWithNullStringsAsEmpty() {
        writer.encodeCommand(buffer, CacheRequestMessageTypes.CREATE_CACHE_MSG_ID, 0L, 0L,
                "corr-2", "cacheB", null, null);

        headerDecoder.wrap(buffer, 0);
        final GatewayCommandDecoder decoder = new GatewayCommandDecoder();
        decoder.wrap(buffer, headerDecoder.encodedLength(), headerDecoder.blockLength(), headerDecoder.version());
        assertEquals("corr-2", decoder.correlationId());
        assertEquals("cacheB", decoder.cacheId());
        assertEquals("", decoder.key());
        assertEquals("", decoder.value());
    }

    @Test
    void encodesSubscribeFrameWithMultipleCacheIds() {
        final List<String> cacheIds = List.of("c1", "c2", "c3");
        writer.encodeSubscribe(buffer, "sub-corr", cacheIds, true, true);

        headerDecoder.wrap(buffer, 0);
        assertEquals(GatewaySubscribeDecoder.TEMPLATE_ID, headerDecoder.templateId());

        final GatewaySubscribeDecoder decoder = new GatewaySubscribeDecoder();
        decoder.wrap(buffer, headerDecoder.encodedLength(), headerDecoder.blockLength(), headerDecoder.version());
        assertEquals(BooleanType.T, decoder.sendSnapshot());
        assertEquals(BooleanType.T, decoder.counters());

        final List<String> decoded = new ArrayList<>();
        final var group = decoder.cacheIds();
        while (group.hasNext()) {
            group.next();
            // Read every field of the entry in schema order (mode, cacheId, key) so the var-data
            // cursor advances correctly across iterations.
            assertEquals(SubscriptionMode.FULL, group.mode());
            decoded.add(group.cacheId());
            assertEquals("", group.key());
        }
        assertEquals(cacheIds, decoded);
        assertEquals("sub-corr", decoder.correlationId());
    }

    @Test
    void encodesUnsubscribeFrame() {
        writer.encodeUnsubscribe(buffer, "unsub-corr", "cacheZ", false);

        headerDecoder.wrap(buffer, 0);
        assertEquals(GatewayUnsubscribeDecoder.TEMPLATE_ID, headerDecoder.templateId());

        final GatewayUnsubscribeDecoder decoder = new GatewayUnsubscribeDecoder();
        decoder.wrap(buffer, headerDecoder.encodedLength(), headerDecoder.blockLength(), headerDecoder.version());
        assertEquals(BooleanType.F, decoder.counters());
        assertEquals("unsub-corr", decoder.correlationId());
        assertEquals("cacheZ", decoder.cacheId());
    }

    @Test
    void subscribeWithEmptyCacheListEncodes() {
        writer.encodeSubscribe(buffer, "corr", List.of(), false, false);
        headerDecoder.wrap(buffer, 0);
        final GatewaySubscribeDecoder decoder = new GatewaySubscribeDecoder();
        decoder.wrap(buffer, headerDecoder.encodedLength(), headerDecoder.blockLength(), headerDecoder.version());
        assertEquals(BooleanType.F, decoder.sendSnapshot());
        assertTrue(decoder.cacheIds().count() == 0);
    }

    @Test
    void encodesSubscribeFrameWithPatchModeAndKey() {
        writer.encodeSubscribe(buffer, "patch-corr", List.of("cacheP"), false, false,
                SubscriptionMode.PATCH, "the-key");

        headerDecoder.wrap(buffer, 0);
        assertEquals(GatewaySubscribeDecoder.TEMPLATE_ID, headerDecoder.templateId());

        final GatewaySubscribeDecoder decoder = new GatewaySubscribeDecoder();
        decoder.wrap(buffer, headerDecoder.encodedLength(), headerDecoder.blockLength(), headerDecoder.version());
        assertEquals(BooleanType.F, decoder.sendSnapshot());
        assertEquals(BooleanType.F, decoder.counters());

        final var group = decoder.cacheIds();
        assertTrue(group.hasNext());
        group.next();
        assertEquals(SubscriptionMode.PATCH, group.mode());
        assertEquals("cacheP", group.cacheId());
        assertEquals("the-key", group.key());
        assertEquals("patch-corr", decoder.correlationId());
    }

    @Test
    void encodesBulkRequestFrame() {
        final CacheOperationRequest add = CacheOperationRequest.builder()
                .operationType(BulkOperationType.ADD_ITEM)
                .ttl(1000L)
                .counterValue(0L)
                .requestId("req-1")
                .cacheId("cacheA")
                .key("k1")
                .value("v1")
                .build();
        final CacheOperationRequest inc = CacheOperationRequest.builder()
                .operationType(BulkOperationType.INCREMENT_COUNTER)
                .ttl(0L)
                .counterValue(5L)
                .requestId("req-2")
                .cacheId("counterA")
                .key("c1")
                .value("")
                .build();

        writer.encodeBulkRequest(buffer, "bulk-corr", List.of(add, inc));

        headerDecoder.wrap(buffer, 0);
        assertEquals(GatewayBulkRequestDecoder.TEMPLATE_ID, headerDecoder.templateId());

        final GatewayBulkRequestDecoder decoder = new GatewayBulkRequestDecoder();
        decoder.wrap(buffer, headerDecoder.encodedLength(), headerDecoder.blockLength(), headerDecoder.version());

        final var group = decoder.operations();
        assertEquals(2, group.count());

        group.next();
        assertEquals(com.bhf.aeroncache.gateway.messages.BulkOperationType.ADD_ITEM, group.operationType());
        assertEquals(1000L, group.ttl());
        assertEquals(0L, group.counterValue());
        assertEquals("req-1", group.requestId());
        assertEquals("cacheA", group.cacheId());
        assertEquals("k1", group.key());
        assertEquals("v1", group.value());

        group.next();
        assertEquals(com.bhf.aeroncache.gateway.messages.BulkOperationType.INCREMENT_COUNTER, group.operationType());
        assertEquals(0L, group.ttl());
        assertEquals(5L, group.counterValue());
        assertEquals("req-2", group.requestId());
        assertEquals("counterA", group.cacheId());
        assertEquals("c1", group.key());
        assertEquals("", group.value());

        assertEquals("bulk-corr", decoder.correlationId());
    }

    @Test
    void bulkResponseEncoderRoundTrips() {
        final MessageHeaderEncoder headerEncoder = new MessageHeaderEncoder();
        final GatewayBulkResponseEncoder encoder = new GatewayBulkResponseEncoder();
        var group = encoder.wrapAndApplyHeader(buffer, 0, headerEncoder).operationsCount(2);
        group.next()
                .status(OperationStatus.SUCCESS)
                .requestId("req-1").cacheId("cacheA").key("k1").value("v1");
        group.next()
                .status(OperationStatus.UNKNOWN_KEY)
                .requestId("req-2").cacheId("cacheA").key("k2").value("");
        encoder.correlationId("bulk-resp-corr");

        headerDecoder.wrap(buffer, 0);
        assertEquals(GatewayBulkResponseDecoder.TEMPLATE_ID, headerDecoder.templateId());

        final GatewayBulkResponseDecoder decoder = new GatewayBulkResponseDecoder();
        decoder.wrap(buffer, headerDecoder.encodedLength(), headerDecoder.blockLength(), headerDecoder.version());

        final var decoded = decoder.operations();
        assertEquals(2, decoded.count());
        decoded.next();
        assertEquals(OperationStatus.SUCCESS, decoded.status());
        assertEquals("req-1", decoded.requestId());
        assertEquals("cacheA", decoded.cacheId());
        assertEquals("k1", decoded.key());
        assertEquals("v1", decoded.value());
        decoded.next();
        assertEquals(OperationStatus.UNKNOWN_KEY, decoded.status());
        assertEquals("req-2", decoded.requestId());
        assertEquals("cacheA", decoded.cacheId());
        assertEquals("k2", decoded.key());
        assertEquals("", decoded.value());
        assertEquals("bulk-resp-corr", decoder.correlationId());
    }

    @Test
    void subscribeAckEncoderRoundTrips() {
        final MessageHeaderEncoder headerEncoder = new MessageHeaderEncoder();
        final GatewaySubscribeAckEncoder encoder = new GatewaySubscribeAckEncoder();
        var group = encoder.wrapAndApplyHeader(buffer, 0, headerEncoder)
                .status(OperationStatus.SUCCESS)
                .cacheIdsCount(2);
        group.next().cacheId("cacheA");
        group.next().cacheId("cacheB");
        encoder.correlationId("ack-corr");

        headerDecoder.wrap(buffer, 0);
        assertEquals(GatewaySubscribeAckDecoder.TEMPLATE_ID, headerDecoder.templateId());

        final GatewaySubscribeAckDecoder decoder = new GatewaySubscribeAckDecoder();
        decoder.wrap(buffer, headerDecoder.encodedLength(), headerDecoder.blockLength(), headerDecoder.version());
        assertEquals(OperationStatus.SUCCESS, decoder.status());

        final List<String> cacheIds = new ArrayList<>();
        for (GatewaySubscribeAckDecoder.CacheIdsDecoder c : decoder.cacheIds()) {
            cacheIds.add(c.cacheId());
        }
        assertEquals(List.of("cacheA", "cacheB"), cacheIds);
        assertEquals("ack-corr", decoder.correlationId());
    }
}
