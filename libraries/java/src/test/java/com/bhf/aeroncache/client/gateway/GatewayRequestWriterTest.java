package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.gateway.messages.BooleanType;
import com.bhf.aeroncache.gateway.messages.GatewayCommandDecoder;
import com.bhf.aeroncache.gateway.messages.GatewaySubscribeDecoder;
import com.bhf.aeroncache.gateway.messages.GatewayUnsubscribeDecoder;
import com.bhf.aeroncache.gateway.messages.MessageHeaderDecoder;
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
            decoded.add(group.cacheId());
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
}
