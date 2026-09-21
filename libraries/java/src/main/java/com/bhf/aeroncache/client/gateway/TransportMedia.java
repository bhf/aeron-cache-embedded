package com.bhf.aeroncache.client.gateway;

import io.aeron.ChannelUriStringBuilder;

/**
 * The Aeron media used for the gateway client's response-channel transport, either UDP (the default)
 * or IPC.
 * <p>
 * Response channels ({@code control-mode=response}) work over both media in Aeron: the driver keys
 * each client's response publication on the request publication's {@code response-correlation-id}
 * regardless of media, so no endpoints are required for the correlation itself. The only difference is
 * that UDP channels carry {@code endpoint}/{@code control-endpoint} addresses, whereas IPC channels
 * have none — those params are omitted for {@link #IPC} (setting an endpoint on an {@code aeron:ipc}
 * channel is rejected by the driver).
 * <p>
 * IPC requires this client and the gateway server to share a single media driver (same host, same
 * {@code aeron.dir}); it is intended for co-located deployments. UDP remains the default for everything
 * else. Mirrors the server's {@code com.bhf.aeroncache.transport.TransportMedia}.
 */
public enum TransportMedia {

    UDP("udp"),
    IPC("ipc");

    private final String media;

    TransportMedia(String media) {
        this.media = media;
    }

    /** @return the Aeron media name ({@code udp} or {@code ipc}). */
    public String media() {
        return media;
    }

    public boolean isIpc() {
        return this == IPC;
    }

    /**
     * Parse a media selection (e.g. from a CLI flag or environment variable), defaulting to {@link #UDP}
     * when {@code value} is {@code null}, blank, or unrecognised.
     */
    public static TransportMedia parse(String value) {
        if (value == null || value.isBlank()) {
            return UDP;
        }
        return switch (value.trim().toLowerCase()) {
            case "ipc" -> IPC;
            default -> UDP;
        };
    }

    // ------------------------------------------------------------------ channel builders
    //
    // Each method returns a fresh builder so the caller can chain the correlation id, e.g.
    // requestPublication(ep).responseCorrelationId(id).build(). For IPC the endpoint/control-endpoint
    // params are omitted - IPC channels have no addresses.

    /**
     * Base builder for the client's request {@link io.aeron.ExclusivePublication}. The caller adds
     * {@code responseCorrelationId(responseSubscription.registrationId())}.
     *
     * @param requestEndpoint the server's request endpoint (host:port); ignored for IPC.
     */
    public ChannelUriStringBuilder requestPublication(String requestEndpoint) {
        final ChannelUriStringBuilder builder = new ChannelUriStringBuilder().media(media);
        if (this == UDP) {
            builder.endpoint(requestEndpoint);
        }
        return builder;
    }

    /**
     * Builder for a gateway server's request {@link io.aeron.Subscription}. Exposed for test doubles
     * (e.g. an in-process loopback gateway) that stand in for the real server.
     *
     * @param requestEndpoint         the endpoint the server listens on (host:port); ignored for IPC.
     * @param responseControlEndpoint the response control endpoint advertised to clients; ignored for IPC.
     */
    public ChannelUriStringBuilder requestSubscription(String requestEndpoint, String responseControlEndpoint) {
        final ChannelUriStringBuilder builder = new ChannelUriStringBuilder().media(media);
        if (this == UDP) {
            builder.endpoint(requestEndpoint).responseEndpoint(responseControlEndpoint);
        }
        return builder;
    }

    /**
     * Builder for a response channel ({@code control-mode=response}) - the client's response
     * {@link io.aeron.Subscription}.
     *
     * @param responseControlEndpoint the response control endpoint (host:port); ignored for IPC.
     */
    public ChannelUriStringBuilder responseChannel(String responseControlEndpoint) {
        final ChannelUriStringBuilder builder = new ChannelUriStringBuilder()
                .media(media)
                .controlMode("response");
        if (this == UDP) {
            builder.controlEndpoint(responseControlEndpoint);
        }
        return builder;
    }
}
