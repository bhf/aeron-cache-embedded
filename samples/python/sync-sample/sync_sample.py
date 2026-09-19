import sys
from aeron_cache.client import AeronCacheClient
from aeron_cache.embedded_cache import EmbeddedAeronCache
from aeron_cache.embedded_counter_cache import EmbeddedCounterCache


def main():
    base_url = "http://localhost:7070"
    ws_url = "http://localhost:7071"
    if len(sys.argv) > 1:
        base_url = sys.argv[1]
        ws_url = sys.argv[2]

    print(f"Starting Sync Sample against {base_url}, ws={ws_url}")

    client = AeronCacheClient(base_url, ws_url)

    try:
        response = client.create_cache("sync-sample-cache")
        print(f"Created cache: {response.cacheId}")
    except Exception:
        pass

    cache = EmbeddedAeronCache(client, "sync-sample-cache")

    print("Putting key 'sync-key' -> 'sync-value'")
    put_response = cache.put("sync-key", "sync-value")
    print(f"Put operation status: {put_response.status if hasattr(put_response, 'status') else 'OK'}")

    get_response = cache.get("sync-key")
    print(f"Read key 'sync-key': {get_response.value or 'not found'}")

    print("Putting key 'timed-key' -> 'timed-value' with 5000ms TTL")
    timed_put_response = cache.put_timed("timed-key", "timed-value", 5000)
    print(f"Timed Put operation status: {timed_put_response.operationStatus or 'OK'}")

    timed_get_response = cache.get("timed-key")
    print(f"Read key 'timed-key': {timed_get_response.value or 'not found'}")

    # --- Counter operations ---
    try:
        counter_resp = client.create_counter_cache("sync-counter-cache")
        print(f"Created counter cache: {counter_resp.cacheId}")
    except Exception:
        pass

    counters = EmbeddedCounterCache(client, "sync-counter-cache")

    print("Putting counter 'requests' -> 10")
    counters.put("requests", 10)
    print(f"Incremented 'requests' by 5 -> {counters.increment('requests', 5).value}")
    print(f"Decremented 'requests' by 3 -> {counters.decrement('requests', 3).value}")
    print(f"Set 'requests' -> {counters.set('requests', 100).value}")
    print(f"Read counter 'requests': {counters.get('requests').value}")

    # --- Inspection & management operations ---
    print("Patching 'sync-key' (deep-merge)")
    client.put_item("sync-sample-cache", "doc", '{"a":1}')
    patch_resp = client.patch_item("sync-sample-cache", "doc", '{"b":2}')
    print(f"Patch status: {patch_resp.operationStatus}")
    print(f"Doc after patch: {client.get_item('sync-sample-cache', 'doc').value}")

    print("Listing all caches:")
    for details in client.get_caches():
        print(f"  - {details.cacheId} ({details.itemCount} items)")

    stats = client.get_stats()
    print(f"Cache stats: caches={stats.totalCachesCount} items={stats.totalItemsCount} ops={stats.totalOpsCount} errors={stats.errorCount}")

    # A timed entry schedules a pending TTL removal timer; get_timers lists all pending timers
    # across both caches and counter caches, each tagged CACHE or COUNTER.
    client.put_timed_item("sync-sample-cache", "expiring", "gone-soon", 600000)
    print("Listing all pending TTL timers:")
    for timer in client.get_timers().timers:
        print(f"  - [{timer.timerType}] {timer.cacheId}/{timer.key} fires at {timer.deadline}")

    print("Listing all counters in 'sync-counter-cache':")
    for item in client.get_counter_items("sync-counter-cache").items:
        print(f"  - {item.key} = {item.value}")

    counter_stats = client.get_counter_stats()
    print(f"Counter stats: caches={counter_stats.totalCachesCount} items={counter_stats.totalItemsCount}")


if __name__ == "__main__":
    main()
