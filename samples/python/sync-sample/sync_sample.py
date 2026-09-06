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


if __name__ == "__main__":
    main()
