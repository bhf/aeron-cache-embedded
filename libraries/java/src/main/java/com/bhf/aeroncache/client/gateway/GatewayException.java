package com.bhf.aeroncache.client.gateway;

import com.bhf.aeroncache.gateway.messages.OperationStatus;

/**
 * Thrown (via a failed future) when the gateway returns a {@code GatewayError} frame for a request.
 */
public class GatewayException extends RuntimeException {

    private final OperationStatus status;

    public GatewayException(OperationStatus status, String message) {
        super("Gateway error [" + status + "]: " + message);
        this.status = status;
    }

    public OperationStatus getStatus() {
        return status;
    }
}
