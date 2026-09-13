package com.bhf.aeroncache.client.bidi;

/**
 * An error frame ({@code {"type":"error", ...}}) correlated to a bidi request. Carries the server's
 * {@code status} and {@code message}.
 */
public class BidiException extends RuntimeException {

    private final String status;

    public BidiException(String status, String message) {
        super(status + ": " + message);
        this.status = status;
    }

    public String getStatus() {
        return status;
    }
}
