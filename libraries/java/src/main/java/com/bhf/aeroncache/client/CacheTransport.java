package com.bhf.aeroncache.client;

import com.bhf.aeroncache.models.CacheUpdateEvent;
import com.bhf.aeroncache.models.CounterResponse;
import com.bhf.aeroncache.models.CounterUpdateEvent;
import com.bhf.aeroncache.models.CreateResponse;
import com.bhf.aeroncache.models.DeleteCacheResponse;
import com.bhf.aeroncache.models.DeleteItemResponse;
import com.bhf.aeroncache.models.GetItemResponse;
import com.bhf.aeroncache.models.PutItemResponse;

import java.util.concurrent.CompletableFuture;
import java.util.function.Consumer;

/**
 * Transport-neutral view of the Aeron Cache operations, so callers (in particular
 * {@link EmbeddedAeronCache} and {@link EmbeddedCounterCache}) can work over either transport:
 * the HTTP+WS {@link AeronCacheClient} or the Aeron
 * {@link com.bhf.aeroncache.client.gateway.AeronGatewayClient}.
 * <p>
 * Streaming subscriptions are exposed as {@link #subscribeCacheUpdates} / {@link #subscribeCounterUpdates},
 * which deliver decoded {@link CacheUpdateEvent}/{@link CounterUpdateEvent} to a {@link Consumer} and return
 * an {@link AutoCloseable} handle that unsubscribes when closed — independent of whether the underlying
 * transport is a WebSocket or an Aeron subscription.
 */
public interface CacheTransport {

    // ------------------------------------------------------------------ cache ops

    CreateResponse createCache(String cacheId) throws Exception;

    PutItemResponse putItem(String cacheId, String key, String value) throws Exception;

    PutItemResponse putTimedItem(String cacheId, String key, String value, long ttl) throws Exception;

    GetItemResponse getItem(String cacheId, String key) throws Exception;

    DeleteItemResponse deleteItem(String cacheId, String key) throws Exception;

    DeleteCacheResponse deleteCache(String cacheId) throws Exception;

    CompletableFuture<CreateResponse> createCacheAsync(String cacheId);

    CompletableFuture<PutItemResponse> putItemAsync(String cacheId, String key, String value);

    CompletableFuture<PutItemResponse> putTimedItemAsync(String cacheId, String key, String value, long ttl);

    CompletableFuture<GetItemResponse> getItemAsync(String cacheId, String key);

    CompletableFuture<DeleteItemResponse> deleteItemAsync(String cacheId, String key);

    CompletableFuture<DeleteCacheResponse> deleteCacheAsync(String cacheId);

    /**
     * Subscribe to streaming updates for a regular cache.
     *
     * @param hydrate  request an initial snapshot of the current contents.
     * @param listener receives decoded {@link CacheUpdateEvent}s.
     * @return a handle that unsubscribes when {@link AutoCloseable#close() closed}.
     */
    AutoCloseable subscribeCacheUpdates(String cacheId, boolean hydrate, Consumer<CacheUpdateEvent> listener);

    // ------------------------------------------------------------------ counter ops

    CreateResponse createCounterCache(String cacheId) throws Exception;

    PutItemResponse putCounter(String cacheId, String key, long value) throws Exception;

    PutItemResponse putTimedCounter(String cacheId, String key, long value, long ttl) throws Exception;

    CounterResponse getCounter(String cacheId, String key) throws Exception;

    CounterResponse incrementCounter(String cacheId, String key, long amount) throws Exception;

    CounterResponse decrementCounter(String cacheId, String key, long amount) throws Exception;

    CounterResponse setCounter(String cacheId, String key, long value) throws Exception;

    DeleteItemResponse deleteCounter(String cacheId, String key) throws Exception;

    DeleteCacheResponse deleteCounterCache(String cacheId) throws Exception;

    CompletableFuture<CreateResponse> createCounterCacheAsync(String cacheId);

    CompletableFuture<PutItemResponse> putCounterAsync(String cacheId, String key, long value);

    CompletableFuture<PutItemResponse> putTimedCounterAsync(String cacheId, String key, long value, long ttl);

    CompletableFuture<CounterResponse> getCounterAsync(String cacheId, String key);

    CompletableFuture<CounterResponse> incrementCounterAsync(String cacheId, String key, long amount);

    CompletableFuture<CounterResponse> decrementCounterAsync(String cacheId, String key, long amount);

    CompletableFuture<CounterResponse> setCounterAsync(String cacheId, String key, long value);

    CompletableFuture<DeleteItemResponse> deleteCounterAsync(String cacheId, String key);

    CompletableFuture<DeleteCacheResponse> deleteCounterCacheAsync(String cacheId);

    /**
     * Subscribe to streaming updates for a counter cache.
     *
     * @param hydrate  request an initial snapshot of the current contents.
     * @param listener receives decoded {@link CounterUpdateEvent}s.
     * @return a handle that unsubscribes when {@link AutoCloseable#close() closed}.
     */
    AutoCloseable subscribeCounterUpdates(String cacheId, boolean hydrate, Consumer<CounterUpdateEvent> listener);

    // ------------------------------------------------------------------ embedded caches

    /**
     * A local-mirroring cache backed by this transport.
     */
    default EmbeddedAeronCache getCache(String cacheId) {
        return new EmbeddedAeronCache(this, cacheId);
    }

    /**
     * A local-mirroring counter cache backed by this transport.
     */
    default EmbeddedCounterCache getCounterCache(String cacheId) {
        return new EmbeddedCounterCache(this, cacheId);
    }
}
