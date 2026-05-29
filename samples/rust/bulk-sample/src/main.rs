use aeron_cache_embedded_client::{
    AeronCacheClient, BulkCacheOpsRequest, CacheOperationRequest, BulkOperationType
};
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let base_url = args.get(1).map(|s| s.as_str()).unwrap_or("http://localhost:7070");
    let ws_url = args.get(2).map(|s| s.as_str()).unwrap_or("ws://localhost:7071");

    println!("Starting Rust Bulk Operations Sample against {}", base_url);

    let client = AeronCacheClient::new(base_url.to_string(), ws_url.to_string());
    let cache_id = "bulk-rust-sample";

    let bulk_request = BulkCacheOpsRequest {
        request_id: Uuid::new_v4().to_string(),
        operations: vec![
            CacheOperationRequest {
                operation_type: BulkOperationType::CreateCache,
                request_id: "op-1".to_string(),
                cache_id: cache_id.to_string(),
                key: None,
                value: None,
                ttl: None,
            },
            CacheOperationRequest {
                operation_type: BulkOperationType::AddItem,
                request_id: "op-2".to_string(),
                cache_id: cache_id.to_string(),
                key: Some("rust-bulk-1".to_string()),
                value: Some("value-1".to_string()),
                ttl: None,
            },
            CacheOperationRequest {
                operation_type: BulkOperationType::GetItem,
                request_id: "op-3".to_string(),
                cache_id: cache_id.to_string(),
                key: Some("rust-bulk-1".to_string()),
                value: None,
                ttl: None,
            },
        ],
    };

    println!("Executing bulk operations...");
    let response = client.bulk_ops_async(&bulk_request).await?;

    println!("Bulk Response ID: {}", response.request_id);
    for op_resp in response.operation_responses {
        println!(
            "  Op {}: status={}, cache={}, key={:?}, value={:?}",
            op_resp.request_id, op_resp.status, op_resp.cache_id, op_resp.key, op_resp.value
        );
    }

    Ok(())
}
