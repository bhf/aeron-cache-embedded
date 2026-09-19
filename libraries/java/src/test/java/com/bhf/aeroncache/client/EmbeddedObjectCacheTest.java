package com.bhf.aeroncache.client;

import com.bhf.aeroncache.models.CacheUpdateEvent;
import com.fasterxml.jackson.databind.node.ObjectNode;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.mockito.ArgumentCaptor;

import java.util.Map;
import java.util.function.Consumer;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.anyBoolean;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.*;

public class EmbeddedObjectCacheTest {

    private CacheTransport mockTransport;
    private EmbeddedObjectCache cache;

    @BeforeEach
    void setUp() {
        mockTransport = mock(CacheTransport.class);
        cache = new EmbeddedObjectCache(mockTransport, "test-cache");
    }

    private static CacheUpdateEvent event(String type, String key, String value) {
        CacheUpdateEvent e = new CacheUpdateEvent();
        e.setEventType(type);
        e.setItemKey(key);
        e.setItemValue(value);
        return e;
    }

    // --- delegation ---

    @Test
    public void testPutSerializesObjectAndDelegates() throws Exception {
        cache.put("doc", Map.of("a", 1));
        verify(mockTransport, times(1)).putItem("test-cache", "doc", "{\"a\":1}");
    }

    @Test
    public void testPatchSerializesFragmentAndDelegates() throws Exception {
        cache.patch("doc", Map.of("b", 2));
        verify(mockTransport, times(1)).patchItem("test-cache", "doc", "{\"b\":2}");
    }

    @Test
    public void testRemoveDelegates() throws Exception {
        cache.remove("doc");
        verify(mockTransport, times(1)).deleteItem("test-cache", "doc");
    }

    @Test
    public void testClearDelegates() throws Exception {
        cache.clear();
        verify(mockTransport, times(1)).deleteCache("test-cache");
    }

    // --- local mirror via subscription ---

    @SuppressWarnings("unchecked")
    private Consumer<CacheUpdateEvent> subscribeAndCaptureListener() {
        ArgumentCaptor<Consumer<CacheUpdateEvent>> captor = ArgumentCaptor.forClass(Consumer.class);
        when(mockTransport.subscribeCacheUpdates(eq("test-cache"), anyBoolean(), captor.capture()))
                .thenReturn(() -> {});
        cache.subscribe((Consumer<CacheUpdateEvent>) null);
        return captor.getValue();
    }

    @Test
    public void testAddItemStoresParsedObject() {
        Consumer<CacheUpdateEvent> listener = subscribeAndCaptureListener();

        listener.accept(event("ADD_ITEM", "doc", "{\"a\":1,\"b\":{\"c\":2}}"));

        ObjectNode stored = cache.getLocal("doc");
        assertEquals(1, stored.get("a").asInt());
        assertEquals(2, stored.get("b").get("c").asInt());
    }

    @Test
    public void testPatchItemDeepMergesInsteadOfOverwriting() {
        Consumer<CacheUpdateEvent> listener = subscribeAndCaptureListener();

        listener.accept(event("ADD_ITEM", "doc", "{\"a\":1,\"b\":{\"c\":2}}"));
        // Patch mode streams only changed fields; the untouched "a" must survive.
        listener.accept(event("PATCH_ITEM", "doc", "{\"b\":{\"d\":3}}"));

        ObjectNode stored = cache.getLocal("doc");
        assertEquals(1, stored.get("a").asInt(), "untouched top-level field must be preserved");
        assertEquals(2, stored.get("b").get("c").asInt(), "untouched nested field must be preserved");
        assertEquals(3, stored.get("b").get("d").asInt(), "new nested field must be merged in");
    }

    @Test
    public void testPatchNullDeletesField() {
        Consumer<CacheUpdateEvent> listener = subscribeAndCaptureListener();

        listener.accept(event("ADD_ITEM", "doc", "{\"a\":1,\"b\":2}"));
        listener.accept(event("PATCH_ITEM", "doc", "{\"b\":null}"));

        ObjectNode stored = cache.getLocal("doc");
        assertEquals(1, stored.get("a").asInt());
        assertFalse(stored.has("b"), "null in a merge-patch must delete the field (RFC 7386)");
    }

    @Test
    public void testPatchOnAbsentKeyStartsFromDelta() {
        Consumer<CacheUpdateEvent> listener = subscribeAndCaptureListener();

        listener.accept(event("PATCH_ITEM", "doc", "{\"a\":1}"));

        assertEquals(1, cache.getLocal("doc").get("a").asInt());
    }

    @Test
    public void testScalarAndArrayFieldsAreReplacedNotMerged() {
        Consumer<CacheUpdateEvent> listener = subscribeAndCaptureListener();

        listener.accept(event("ADD_ITEM", "doc", "{\"n\":1,\"list\":[1,2,3]}"));
        listener.accept(event("PATCH_ITEM", "doc", "{\"n\":9,\"list\":[4]}"));

        ObjectNode stored = cache.getLocal("doc");
        assertEquals(9, stored.get("n").asInt());
        assertEquals(1, stored.get("list").size());
        assertEquals(4, stored.get("list").get(0).asInt());
    }

    @Test
    public void testRemoveAndClearEvents() {
        Consumer<CacheUpdateEvent> listener = subscribeAndCaptureListener();

        listener.accept(event("ADD_ITEM", "doc", "{\"a\":1}"));
        listener.accept(event("REMOVE_ITEM", "doc", null));
        assertNull(cache.getLocal("doc"));

        listener.accept(event("ADD_ITEM", "doc", "{\"a\":1}"));
        listener.accept(event("CLEAR_CACHE", null, null));
        assertTrue(cache.getLocalCache().isEmpty());
    }

    @Test
    public void testGetLocalAsDeserializesToPojo() {
        Consumer<CacheUpdateEvent> listener = subscribeAndCaptureListener();
        listener.accept(event("ADD_ITEM", "doc", "{\"name\":\"widget\",\"qty\":5}"));

        Item item = cache.getLocalAs("doc", Item.class);
        assertEquals("widget", item.name);
        assertEquals(5, item.qty);
        assertNull(cache.getLocalAs("missing", Item.class));
    }

    @Test
    @SuppressWarnings("unchecked")
    public void testListenerStillReceivesEvent() {
        ArgumentCaptor<Consumer<CacheUpdateEvent>> captor = ArgumentCaptor.forClass(Consumer.class);
        when(mockTransport.subscribeCacheUpdates(eq("test-cache"), anyBoolean(), captor.capture()))
                .thenReturn(() -> {});
        boolean[] seen = {false};
        cache.subscribe(e -> seen[0] = true);
        captor.getValue().accept(event("ADD_ITEM", "doc", "{\"a\":1}"));
        assertTrue(seen[0]);
    }

    static class Item {
        public String name;
        public int qty;
    }
}
