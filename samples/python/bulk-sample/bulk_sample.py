import sys
import uuid
from aeron_cache.client import AeronCacheClient
from aeron_cache.models import BulkCacheOpsRequest, CacheOperationRequest, BulkOperationType


def main():
    base_url = "http://localhost:7070"
    ws_url = "http://localhost:7071"
    if len(sys.argv) > 1:
        base_url = sys.argv[1]
        ws_url = sys.argv[2]

    print(f"Starting Bulk Operations Sample against {base_url}")

    client = AeronCacheClient(base_url, ws_url)
    cache_id = "bulk-sample-cache"

    # Define a series of operations to be executed in bulk
    operations = [
        CacheOperationRequest(
            operationType=BulkOperationType.CREATE_CACHE,
            requestId="op-1",
            cacheId=cache_id
        ),
        CacheOperationRequest(
            operationType=BulkOperationType.ADD_ITEM,
            requestId="op-2",
            cacheId=cache_id,
            key="bulk-key-1",
            value="bulk-value-1"
        ),
        CacheOperationRequest(
            operationType=BulkOperationType.ADD_ITEM,
            requestId="op-3",
            cacheId=cache_id,
            key="bulk-key-2",
            value="bulk-value-2"
        ),
        CacheOperationRequest(
            operationType=BulkOperationType.GET_ITEM,
            requestId="op-4",
            cacheId=cache_id,
            key="bulk-key-1"
        )
    ]

    bulk_request = BulkCacheOpsRequest(
        requestId=str(uuid.uuid4()),
        operations=operations
    )

    print(f"Executing {len(operations)} operations in bulk...")
    response = client.bulk_ops(bulk_request)

    print(f"Bulk Request ID: {response.requestId}")
    for op_resp in response.operationResponses:
        print(f"  Operation {op_resp.requestId}: status={op_resp.status}, cache={op_resp.cacheId}, key={op_resp.key}, value={op_resp.value}")


if __name__ == "__main__":
    main()
