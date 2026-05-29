package com.bhf.aeroncache.samples;

import com.bhf.aeroncache.client.AeronCacheClient;
import com.bhf.aeroncache.models.*;

import java.util.Arrays;
import java.util.UUID;

public class BulkSample {
    public static void main(String[] args) throws Exception {
        String baseUrl = args.length > 0 ? args[0] : "http://localhost:7070";
        String wsUrl = args.length > 1 ? args[1] : "ws://localhost:7071";

        System.out.println("Starting Java Bulk Operations Sample against " + baseUrl);

        AeronCacheClient client = new AeronCacheClient(baseUrl, wsUrl);
        String cacheId = "bulk-java-sample";

        BulkCacheOpsRequest bulkRequest = BulkCacheOpsRequest.builder()
                .requestId(UUID.randomUUID().toString())
                .operations(Arrays.asList(
                        CacheOperationRequest.builder()
                                .operationType(BulkOperationType.CREATE_CACHE)
                                .requestId("op-1")
                                .cacheId(cacheId)
                                .build(),
                        CacheOperationRequest.builder()
                                .operationType(BulkOperationType.ADD_ITEM)
                                .requestId("op-2")
                                .cacheId(cacheId)
                                .key("java-bulk-1")
                                .value("value-1")
                                .build(),
                        CacheOperationRequest.builder()
                                .operationType(BulkOperationType.GET_ITEM)
                                .requestId("op-3")
                                .cacheId(cacheId)
                                .key("java-bulk-1")
                                .build()
                ))
                .build();

        System.out.println("Executing bulk operations...");
        BulkCacheOpsResponse response = client.bulkOps(bulkRequest);

        System.out.println("Bulk Response ID: " + response.getRequestId());
        for (CacheOperationResponse opResp : response.getOperationResponses()) {
            System.out.printf("  Op %s: status=%s, cache=%s, key=%s, value=%s%n",
                    opResp.getRequestId(), opResp.getStatus(), opResp.getCacheId(), opResp.getKey(), opResp.getValue());
        }
    }
}
