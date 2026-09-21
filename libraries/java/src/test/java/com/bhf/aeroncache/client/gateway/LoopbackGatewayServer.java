package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.gateway.messages.GatewayCommandDecoder;
import com.bhf.aeroncache.gateway.messages.GatewayCommandResponseEncoder;
import com.bhf.aeroncache.gateway.messages.MessageHeaderDecoder;
import com.bhf.aeroncache.gateway.messages.MessageHeaderEncoder;
import com.bhf.aeroncache.gateway.messages.OperationStatus;
import io.aeron.Aeron;
import io.aeron.ChannelUriStringBuilder;
import io.aeron.FragmentAssembler;
import io.aeron.Image;
import io.aeron.Publication;
import io.aeron.Subscription;
import io.aeron.logbuffer.Header;
import org.agrona.DirectBuffer;
import org.agrona.ExpandableArrayBuffer;
import org.agrona.MutableDirectBuffer;
import org.agrona.collections.Long2ObjectHashMap;
import org.agrona.concurrent.Agent;
import org.agrona.concurrent.UnsafeBuffer;

import java.util.ArrayDeque;
import java.util.Deque;

/**
 * A minimal in-process gateway server double used only to prove that {@link GatewayClient} round-trips a
 * command over a given {@link TransportMedia} (in particular, IPC). It mirrors the production gateway's
 * Aeron response-channel wiring (a request subscription plus one {@code control-mode=response} publication
 * per client image, keyed by {@link Image#correlationId()}) but acknowledges every command with a canned
 * {@code SUCCESS} response rather than talking to a real cache backend.
 * <p>
 * Runs on a single {@link org.agrona.concurrent.AgentRunner} thread. Responses are queued and only offered
 * once the per-session response publication is connected.
 */
class LoopbackGatewayServer implements Agent {

    private static final int FRAGMENT_LIMIT = 10;

    private final Aeron aeron;
    private final int requestStreamId;
    private final int responseStreamId;
    private final ChannelUriStringBuilder requestUriBuilder;
    private final ChannelUriStringBuilder responseUriBuilder;
    private final Long2ObjectHashMap<Publication> sessions = new Long2ObjectHashMap<>();
    private final Deque<PendingFrame> pending = new ArrayDeque<>();

    private final MessageHeaderEncoder headerEncoder = new MessageHeaderEncoder();
    private final MessageHeaderDecoder headerDecoder = new MessageHeaderDecoder();
    private final GatewayCommandDecoder commandDecoder = new GatewayCommandDecoder();
    private final GatewayCommandResponseEncoder commandResponseEncoder = new GatewayCommandResponseEncoder();
    private final MutableDirectBuffer scratch = new ExpandableArrayBuffer(4096);
    private final FragmentAssembler fragmentAssembler = new FragmentAssembler(this::onFragment);

    private Subscription subscription;

    LoopbackGatewayServer(Aeron aeron, TransportMedia media, String requestEndpoint, String responseControl,
                          int requestStreamId, int responseStreamId) {
        this.aeron = aeron;
        this.requestStreamId = requestStreamId;
        this.responseStreamId = responseStreamId;
        this.requestUriBuilder = media.requestSubscription(requestEndpoint, responseControl);
        this.responseUriBuilder = media.responseChannel(responseControl);
    }

    @Override
    public int doWork() {
        int work = 0;
        if (subscription == null) {
            subscription = aeron.addSubscription(requestUriBuilder.build(), requestStreamId);
            work++;
        }
        work += drainPending();
        work += subscription.poll(fragmentAssembler, FRAGMENT_LIMIT);
        return work;
    }

    @Override
    public String roleName() {
        return "Loopback-Gateway-Server";
    }

    @Override
    public void onClose() {
        sessions.values().forEach(Publication::close);
        sessions.clear();
        if (subscription != null) {
            subscription.close();
        }
    }

    private int drainPending() {
        int work = 0;
        int remaining = pending.size();
        while (remaining-- > 0) {
            final PendingFrame frame = pending.poll();
            if (frame == null) {
                break;
            }
            if (frame.publication.isConnected() && frame.publication.offer(frame.buffer, 0, frame.length) > 0) {
                work++;
            } else {
                pending.addLast(frame);
            }
        }
        return work;
    }

    private Publication responseFor(Image image) {
        final long correlationId = image.correlationId();
        Publication publication = sessions.get(correlationId);
        if (publication == null) {
            publication = aeron.addPublication(
                    responseUriBuilder.responseCorrelationId(correlationId).build(), responseStreamId);
            sessions.put(correlationId, publication);
        }
        return publication;
    }

    private void onFragment(DirectBuffer buffer, int offset, int length, Header header) {
        final Image image = (Image) header.context();
        final Publication response = responseFor(image);

        headerDecoder.wrap(buffer, offset);
        final int templateId = headerDecoder.templateId();
        final int blockLength = headerDecoder.blockLength();
        final int version = headerDecoder.version();
        final int bodyOffset = offset + headerDecoder.encodedLength();

        if (templateId == GatewayCommandDecoder.TEMPLATE_ID) {
            commandDecoder.wrap(buffer, bodyOffset, blockLength, version);
            final String correlationId = commandDecoder.correlationId();
            final String cacheId = commandDecoder.cacheId();
            final String key = commandDecoder.key();
            enqueueCommandResponse(response, correlationId, cacheId, key, "");
        }
    }

    private void enqueueCommandResponse(Publication response, String correlationId, String cacheId,
                                        String key, String value) {
        commandResponseEncoder.wrapAndApplyHeader(scratch, 0, headerEncoder)
                .status(OperationStatus.SUCCESS)
                .correlationId(correlationId)
                .cacheId(cacheId)
                .key(key)
                .value(value);
        enqueue(response, commandResponseEncoder.limit());
    }

    private void enqueue(Publication response, int length) {
        final byte[] bytes = new byte[length];
        scratch.getBytes(0, bytes, 0, length);
        pending.addLast(new PendingFrame(response, new UnsafeBuffer(bytes), length));
    }

    private record PendingFrame(Publication publication, DirectBuffer buffer, int length) {
    }
}
