import sys
from aeron_cache.client import AeronCacheClient
from aeron_cache.embedded_cache import EmbeddedAeronCache


def main():
    base_url = "http://localhost:7070"
    ws_url = "http://localhost:7071"
    if len(sys.argv) > 1:
        base_url = sys.argv[1]
        ws_url = sys.argv[2]

    print(f"Starting Sync Sample against {base_url}, ws={ws_url}")

    client = AeronCacheClient(base_url, ws_url)

    try:
        client.create_cache("sync-sample-cache")
    except:
        pass

    cache = EmbeddedAeronCache(client, "sync-sample-cache")

    print("Putting key 'sync-key' -> 'sync-value'")
    cache.put("sync-key", "sync-value")
    print("Put operation completed.")

    val = cache.get("sync-key")
    print(f"Read key 'sync-key': {val or 'not found'}")


if __name__ == "__main__":
    main()
