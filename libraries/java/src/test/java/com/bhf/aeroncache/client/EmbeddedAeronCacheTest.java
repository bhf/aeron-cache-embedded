package com.bhf.aeroncache.client;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.mockito.ArgumentCaptor;
import java.net.http.WebSocket;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.*;

public class EmbeddedAeronCacheTest {

    private AeronCacheClient mockClient;
    private EmbeddedAeronCache cache;

    @BeforeEach
    void setUp() {
        mockClient = mock(AeronCacheClient.class);
        cache = new EmbeddedAeronCache(mockClient, "test-cache");
    }

    @Test
    public void testPutDelegatesToClient() throws Exception {
        cache.put("my-key", "my-value");
        verify(mockClient, times(1)).putItem("test-cache", "my-key", "my-value");
    }

    @Test
    public void testPutTimedDelegatesToClient() throws Exception {
        cache.putTimed("my-key", "my-value", 1000L);
        verify(mockClient, times(1)).putTimedItem("test-cache", "my-key", "my-value", 1000L);
    }

    @Test
    public void testRemoveDelegatesToClient() throws Exception {
        cache.remove("my-key");
        verify(mockClient, times(1)).deleteItem("test-cache", "my-key");
    }
    
    @Test
    public void testClearDelegatesToClient() throws Exception {
        cache.clear();
        verify(mockClient, times(1)).deleteCache("test-cache");
    }

    @Test
    public void testSubscribeDelegatesToClient() {
        ReconnectingWebSocket mockWebSocket = mock(ReconnectingWebSocket.class);
        when(mockClient.subscribe(eq("test-cache"), anyBoolean(), any(AeronCacheSubscriber.class))).thenReturn(mockWebSocket);

        AeronCacheSubscriber subscriber = mock(AeronCacheSubscriber.class);
        ReconnectingWebSocket returnedWs = cache.subscribe(subscriber);

        assertEquals(mockWebSocket, returnedWs);
        verify(mockClient, times(1)).subscribe(eq("test-cache"), eq(false), eq(subscriber));
    }
    
    @Test
    public void testUpdatesLocalCacheViaSubscriptionAndReturnsLocalValue() {
        ReconnectingWebSocket mockWebSocket = mock(ReconnectingWebSocket.class);
        ArgumentCaptor<AeronCacheSubscriber> captor = ArgumentCaptor.forClass(AeronCacheSubscriber.class);
        when(mockClient.subscribe(eq("test-cache"), anyBoolean(), captor.capture())).thenReturn(mockWebSocket);

        cache.subscribe(new AeronCacheSubscriber() {});

        AeronCacheSubscriber registeredSubscriber = captor.getValue();
        
        WebSocket baseMockWs = mock(WebSocket.class);

        // Simulate ADD_ITEM
        registeredSubscriber.onText(baseMockWs, "{\"eventType\":\"ADD_ITEM\",\"itemKey\":\"my-key\",\"itemValue\":\"my-value\"}", true);
        assertEquals("my-value", cache.getLocal("my-key"));
        
        // Simulate REMOVE_ITEM
        registeredSubscriber.onText(baseMockWs, "{\"eventType\":\"REMOVE_ITEM\",\"itemKey\":\"my-key\"}", true);
        assertNull(cache.getLocal("my-key"));
    }
}
