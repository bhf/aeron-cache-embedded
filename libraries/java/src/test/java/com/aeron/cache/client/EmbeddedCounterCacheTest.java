package com.aeron.cache.client;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.mockito.ArgumentCaptor;
import java.net.http.WebSocket;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.*;

public class EmbeddedCounterCacheTest {

    private AeronCacheClient mockClient;
    private EmbeddedCounterCache cache;

    @BeforeEach
    void setUp() {
        mockClient = mock(AeronCacheClient.class);
        cache = new EmbeddedCounterCache(mockClient, "test-counter");
    }

    @Test
    public void testPutDelegatesToClient() throws Exception {
        cache.put("hits", 42);
        verify(mockClient, times(1)).putCounter("test-counter", "hits", 42L);
    }

    @Test
    public void testIncrementDelegatesToClient() throws Exception {
        cache.increment("hits", 1);
        verify(mockClient, times(1)).incrementCounter("test-counter", "hits", 1L);
    }

    @Test
    public void testDecrementDelegatesToClient() throws Exception {
        cache.decrement("hits", 1);
        verify(mockClient, times(1)).decrementCounter("test-counter", "hits", 1L);
    }

    @Test
    public void testSetDelegatesToClient() throws Exception {
        cache.set("hits", 100);
        verify(mockClient, times(1)).setCounter("test-counter", "hits", 100L);
    }

    @Test
    public void testClearDelegatesToClient() throws Exception {
        cache.clear();
        verify(mockClient, times(1)).deleteCounterCache("test-counter");
    }

    @Test
    public void testUpdatesLocalCacheViaSubscription() {
        ReconnectingWebSocket mockWebSocket = mock(ReconnectingWebSocket.class);
        ArgumentCaptor<CounterCacheSubscriber> captor = ArgumentCaptor.forClass(CounterCacheSubscriber.class);
        when(mockClient.subscribeCounter(eq("test-counter"), captor.capture())).thenReturn(mockWebSocket);

        cache.subscribe(new CounterCacheSubscriber() {});

        CounterCacheSubscriber registeredSubscriber = captor.getValue();
        WebSocket baseMockWs = mock(WebSocket.class);

        // Simulate ADD_ITEM
        registeredSubscriber.onText(baseMockWs, "{\"eventType\":\"ADD_ITEM\",\"itemKey\":\"hits\",\"itemValue\":42}", true);
        assertEquals(Long.valueOf(42L), cache.getLocal("hits"));

        // Zero is a valid counter value and must be stored, not treated as missing.
        registeredSubscriber.onText(baseMockWs, "{\"eventType\":\"ADD_ITEM\",\"itemKey\":\"hits\",\"itemValue\":0}", true);
        assertEquals(Long.valueOf(0L), cache.getLocal("hits"));

        // Simulate REMOVE_ITEM
        registeredSubscriber.onText(baseMockWs, "{\"eventType\":\"REMOVE_ITEM\",\"itemKey\":\"hits\"}", true);
        assertNull(cache.getLocal("hits"));
    }
}
