use reqwest::{Client as ReqwestClient, blocking::Client as BlockingClient};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tungstenite::{connect};
use url::Url;

pub mod embedded_cache;
pub use embedded_cache::EmbeddedAeronCache;

pub mod embedded_counter_cache;
pub use embedded_counter_cache::EmbeddedCounterCache;

// Generated SBE codecs for the Aeron gateway wire protocol (from sbe/gateway-schema.xml).
#[allow(dead_code)]
pub mod gateway_messages;

// Aeron gateway transport (SBE over Aeron response channels).
pub mod gateway;
pub use gateway::AeronGatewayClient;

// Bidirectional WebSocket transport (JSON over a single /api/ws/v1/bidi connection).
pub mod bidi;
pub use bidi::{AeronBidiClient, BidiSubscription, StatEntry};

// Transport-neutral abstraction + embedded caches that work over HTTP+WS or Aeron.
pub mod transport;
pub use transport::{CacheSubscription, CacheTransport};

pub mod unified_cache;
pub use unified_cache::{EmbeddedCache, EmbeddedCounters, EmbeddedObjects};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheItem {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GetCacheResponse {
    pub cache_id: String,
    pub operation_status: String,
    pub items: Vec<CacheItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClearCacheResponse {
    pub cache_id: String,
    pub operation_status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateRequest {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateResponse {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    #[serde(rename = "operationStatus")]
    pub operation_status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PutItemRequest {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PutTimedItemRequest {
    pub key: String,
    pub value: String,
    pub ttl: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PutItemResponse {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    pub key: String,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(rename = "operationStatus")]
    pub operation_status: String,
}

fn default_status() -> String {
    "OK".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetItemResponse {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    pub key: String,
    pub value: String,
    #[serde(rename = "operationStatus")]
    pub operation_status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteItemResponse {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    pub key: String,
    #[serde(rename = "operationStatus")]
    pub operation_status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteCacheResponse {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    #[serde(rename = "operationStatus")]
    pub operation_status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheUpdateEvent {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "itemKey")]
    pub item_key: Option<String>,
    #[serde(rename = "itemValue")]
    pub item_value: Option<String>,
    #[serde(rename = "requestId")]
    pub request_id: String,
}

// --- Counters ---

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CounterResponse {
    pub cache_id: String,
    pub key: String,
    #[serde(default)]
    pub value: i64,
    pub operation_status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CounterUpdateEvent {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "itemKey")]
    pub item_key: Option<String>,
    #[serde(rename = "itemValue")]
    pub item_value: Option<i64>,
    #[serde(rename = "requestId")]
    pub request_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PutCounterRequest {
    pub key: String,
    pub value: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PutTimedCounterRequest {
    pub key: String,
    pub value: i64,
    pub ttl: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CounterAmountRequest {
    pub key: String,
    pub amount: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BulkOperationType {
    None,
    CreateCache,
    AddItem,
    RemoveItem,
    ClearCache,
    GetItem,
    DeleteCache,
    PatchItem,
    CreateCounterCache,
    AddCounter,
    RemoveCounter,
    ClearCounterCache,
    GetCounter,
    DeleteCounterCache,
    IncrementCounter,
    DecrementCounter,
    SetCounter,
    CancelItem,
    CancelCounter,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CacheOperationRequest {
    pub operation_type: BulkOperationType,
    pub request_id: String,
    pub cache_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counter_value: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkCacheOpsRequest {
    pub request_id: String,
    pub operations: Vec<CacheOperationRequest>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CacheOperationResponse {
    pub request_id: String,
    pub status: String,
    pub cache_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkCacheOpsResponse {
    pub request_id: String,
    pub operation_responses: Vec<CacheOperationResponse>,
}

// --- Inspection & management models ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PatchItemRequest {
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PatchItemResponse {
    pub cache_id: String,
    pub key: String,
    pub operation_status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CancelItemRemovalResponse {
    pub cache_id: String,
    pub key: String,
    pub operation_status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CacheDetails {
    pub cache_id: String,
    pub item_count: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatsResponse {
    pub total_ops_count: i32,
    pub total_caches_count: i32,
    pub total_items_count: i32,
    pub error_count: i32,
}

/// A single pending TTL removal timer, as returned by the `getTimers` operation. `timer_type` is
/// `"CACHE"` or `"COUNTER"`; `deadline` is the epoch time (millis) at which removal is scheduled.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerInfo {
    pub timer_type: String,
    pub cache_id: String,
    pub key: String,
    pub deadline: i64,
}

/// The response from `GET /api/v1/timers`: all pending TTL removal timers across caches and counters.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GetTimersResponse {
    pub operation_status: String,
    #[serde(default)]
    pub timers: Vec<TimerInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CounterItem {
    pub key: String,
    pub value: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GetCountersResponse {
    pub cache_id: String,
    pub operation_status: String,
    pub items: Vec<CounterItem>,
}

pub struct AeronCacheClient {
    pub base_url: String,
    pub ws_url: String,
    async_client: ReqwestClient,
    sync_client_cell: std::sync::OnceLock<BlockingClient>,
}

impl AeronCacheClient {
    pub fn new(base_url: String, ws_url: String) -> Self {
        AeronCacheClient {
            base_url,
            ws_url,
            async_client: ReqwestClient::new(),
            sync_client_cell: std::sync::OnceLock::new(),
        }
    }

    fn get_sync_client(&self) -> &BlockingClient {
        self.sync_client_cell.get_or_init(|| BlockingClient::new())
    }

    // --- Sync Operations ---

    pub fn create_cache(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache", self.base_url);
        let req = CreateRequest { cache_id: cache_id.to_string() };
        let resp = self.get_sync_client().post(&url)
            .json(&req)
            .send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        let data = resp.json::<CreateResponse>()?;
        Ok(data)
    }

    pub fn put_item(&self, cache_id: &str, key: &str, value: &str) -> Result<PutItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}", self.base_url, cache_id);
        let req = PutItemRequest { key: key.to_string(), value: value.to_string() };
        let resp = self.get_sync_client().post(&url)
            .json(&req)
            .send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        let data = resp.json::<PutItemResponse>()?;
        Ok(data)
    }

    pub fn put_timed_item(&self, cache_id: &str, key: &str, value: &str, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/timed/{}", self.base_url, cache_id);
        let req = PutTimedItemRequest { key: key.to_string(), value: value.to_string(), ttl };
        let resp = self.get_sync_client().post(&url)
            .json(&req)
            .send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        let data = resp.json::<PutItemResponse>()?;
        Ok(data)
    }

    pub fn get_item(&self, cache_id: &str, key: &str) -> Result<GetItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}/{}", self.base_url, cache_id, key);
        let resp = self.get_sync_client().get(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        let data = resp.json::<GetItemResponse>()?;
        Ok(data)
    }

    pub fn delete_item(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}/{}", self.base_url, cache_id, key);
        let resp = self.get_sync_client().delete(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        let data = resp.json::<DeleteItemResponse>()?;
        Ok(data)
    }
    
    pub fn delete_cache(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}", self.base_url, cache_id);
        let resp = self.get_sync_client().delete(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        let data = resp.json::<DeleteCacheResponse>()?;
        Ok(data)
    }

    pub fn bulk_ops(&self, req: &BulkCacheOpsRequest) -> Result<BulkCacheOpsResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/bulkops", self.base_url);
        let resp = self.get_sync_client().post(&url)
            .json(req)
            .send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        let data = resp.json::<BulkCacheOpsResponse>()?;
        Ok(data)
    }

    // --- Async Operations ---

    pub async fn create_cache_async(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache", self.base_url);
        let req = CreateRequest { cache_id: cache_id.to_string() };
        let resp = self.async_client.post(&url)
            .json(&req)
            .send()
            .await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        let data = resp.json::<CreateResponse>().await?;
        Ok(data)
    }

    pub async fn put_item_async(&self, cache_id: &str, key: &str, value: &str) -> Result<PutItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}", self.base_url, cache_id);
        let req = PutItemRequest { key: key.to_string(), value: value.to_string() };
        let resp = self.async_client.post(&url)
            .json(&req)
            .send()
            .await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        let data = resp.json::<PutItemResponse>().await?;
        Ok(data)
    }

    pub async fn put_timed_item_async(&self, cache_id: &str, key: &str, value: &str, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/timed/{}", self.base_url, cache_id);
        let req = PutTimedItemRequest { key: key.to_string(), value: value.to_string(), ttl };
        let resp = self.async_client.post(&url)
            .json(&req)
            .send()
            .await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        let data = resp.json::<PutItemResponse>().await?;
        Ok(data)
    }

    pub async fn get_item_async(&self, cache_id: &str, key: &str) -> Result<GetItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}/{}", self.base_url, cache_id, key);
        let resp = self.async_client.get(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        let data = resp.json::<GetItemResponse>().await?;
        Ok(data)
    }

    pub async fn delete_item_async(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}/{}", self.base_url, cache_id, key);
        let resp = self.async_client.delete(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        let data = resp.json::<DeleteItemResponse>().await?;
        Ok(data)
    }
    
    pub async fn delete_cache_async(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}", self.base_url, cache_id);
        let resp = self.async_client.delete(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        let data = resp.json::<DeleteCacheResponse>().await?;
        Ok(data)
    }

    pub async fn bulk_ops_async(&self, req: &BulkCacheOpsRequest) -> Result<BulkCacheOpsResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/bulkops", self.base_url);
        let resp = self.async_client.post(&url)
            .json(req)
            .send()
            .await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        let data = resp.json::<BulkCacheOpsResponse>().await?;
        Ok(data)
    }

    
    pub fn get_cache_items(&self, cache_id: &str) -> Result<GetCacheResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}", self.base_url, cache_id);
        let response = self.get_sync_client().get(&url).send()?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND && response.status() != reqwest::StatusCode::BAD_REQUEST {
            return Err(format!("Http Error: {}", response.status()).into());
        }

        let resp_body = response.json::<GetCacheResponse>()?;
        Ok(resp_body)
    }

    pub fn clear_cache(&self, cache_id: &str) -> Result<ClearCacheResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}", self.base_url, cache_id);
        let response = self.get_sync_client().patch(&url).send()?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND && response.status() != reqwest::StatusCode::BAD_REQUEST {
            return Err(format!("Http Error: {}", response.status()).into());
        }

        let resp_body = response.json::<ClearCacheResponse>()?;
        Ok(resp_body)
    }

    pub async fn get_cache_items_async(&self, cache_id: &str) -> Result<GetCacheResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}", self.base_url, cache_id);
        let response = self.async_client.get(&url).send().await?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND && response.status() != reqwest::StatusCode::BAD_REQUEST {
            return Err(format!("Http Error: {}", response.status()).into());
        }

        let resp_body = response.json::<GetCacheResponse>().await?;
        Ok(resp_body)
    }

    pub async fn clear_cache_async(&self, cache_id: &str) -> Result<ClearCacheResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}", self.base_url, cache_id);
        let response = self.async_client.patch(&url).send().await?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND && response.status() != reqwest::StatusCode::BAD_REQUEST {
            return Err(format!("Http Error: {}", response.status()).into());
        }

        let resp_body = response.json::<ClearCacheResponse>().await?;
        Ok(resp_body)
    }

    // --- Additional Cache Operations (Sync) ---

    pub fn patch_item(&self, cache_id: &str, key: &str, value: &str) -> Result<PatchItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}/{}", self.base_url, cache_id, key);
        let req = PatchItemRequest { value: value.to_string() };
        let resp = self.get_sync_client().patch(&url).json(&req).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<PatchItemResponse>()?)
    }

    pub fn cancel_item_removal(&self, cache_id: &str, key: &str) -> Result<CancelItemRemovalResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}/{}/cancel-removal", self.base_url, cache_id, key);
        let resp = self.get_sync_client().post(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<CancelItemRemovalResponse>()?)
    }

    pub fn get_caches(&self) -> Result<Vec<CacheDetails>, Box<dyn Error>> {
        let url = format!("{}/api/v1/caches", self.base_url);
        let resp = self.get_sync_client().get(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<Vec<CacheDetails>>()?)
    }

    pub fn get_stats(&self) -> Result<CacheStatsResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/stats", self.base_url);
        let resp = self.get_sync_client().get(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<CacheStatsResponse>()?)
    }

    pub fn get_timers(&self) -> Result<GetTimersResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/timers", self.base_url);
        let resp = self.get_sync_client().get(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<GetTimersResponse>()?)
    }

    // --- Additional Cache Operations (Async) ---

    pub async fn patch_item_async(&self, cache_id: &str, key: &str, value: &str) -> Result<PatchItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}/{}", self.base_url, cache_id, key);
        let req = PatchItemRequest { value: value.to_string() };
        let resp = self.async_client.patch(&url).json(&req).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<PatchItemResponse>().await?)
    }

    pub async fn cancel_item_removal_async(&self, cache_id: &str, key: &str) -> Result<CancelItemRemovalResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}/{}/cancel-removal", self.base_url, cache_id, key);
        let resp = self.async_client.post(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<CancelItemRemovalResponse>().await?)
    }

    pub async fn get_caches_async(&self) -> Result<Vec<CacheDetails>, Box<dyn Error>> {
        let url = format!("{}/api/v1/caches", self.base_url);
        let resp = self.async_client.get(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<Vec<CacheDetails>>().await?)
    }

    pub async fn get_stats_async(&self) -> Result<CacheStatsResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/stats", self.base_url);
        let resp = self.async_client.get(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<CacheStatsResponse>().await?)
    }

    pub async fn get_timers_async(&self) -> Result<GetTimersResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/timers", self.base_url);
        let resp = self.async_client.get(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<GetTimersResponse>().await?)
    }

    pub fn get_cache(&self, cache_id: &str) -> EmbeddedAeronCache<'_> {
        EmbeddedAeronCache::new(self, cache_id.to_string())
    }

    // --- Counter Operations (Sync) ---

    pub fn create_counter_cache(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/", self.base_url);
        let req = CreateRequest { cache_id: cache_id.to_string() };
        let resp = self.get_sync_client().post(&url).json(&req).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<CreateResponse>()?)
    }

    pub fn put_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}", self.base_url, cache_id);
        let req = PutCounterRequest { key: key.to_string(), value };
        let resp = self.get_sync_client().post(&url).json(&req).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<PutItemResponse>()?)
    }

    pub fn put_timed_counter(&self, cache_id: &str, key: &str, value: i64, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/timed/{}", self.base_url, cache_id);
        let req = PutTimedCounterRequest { key: key.to_string(), value, ttl };
        let resp = self.get_sync_client().post(&url).json(&req).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<PutItemResponse>()?)
    }

    pub fn get_counter(&self, cache_id: &str, key: &str) -> Result<CounterResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}/{}", self.base_url, cache_id, key);
        let resp = self.get_sync_client().get(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<CounterResponse>()?)
    }

    pub fn delete_counter(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}/{}", self.base_url, cache_id, key);
        let resp = self.get_sync_client().delete(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<DeleteItemResponse>()?)
    }

    pub fn delete_counter_cache(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}", self.base_url, cache_id);
        let resp = self.get_sync_client().delete(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<DeleteCacheResponse>()?)
    }

    pub fn increment_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.counter_amount_op("increment", cache_id, key, amount)
    }

    pub fn decrement_counter(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.counter_amount_op("decrement", cache_id, key, amount)
    }

    fn counter_amount_op(&self, op: &str, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}/{}", self.base_url, op, cache_id);
        let req = CounterAmountRequest { key: key.to_string(), amount };
        let resp = self.get_sync_client().post(&url).json(&req).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<CounterResponse>()?)
    }

    pub fn set_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/set/{}", self.base_url, cache_id);
        let req = PutCounterRequest { key: key.to_string(), value };
        let resp = self.get_sync_client().post(&url).json(&req).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<CounterResponse>()?)
    }

    // --- Counter Operations (Async) ---

    pub async fn create_counter_cache_async(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/", self.base_url);
        let req = CreateRequest { cache_id: cache_id.to_string() };
        let resp = self.async_client.post(&url).json(&req).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<CreateResponse>().await?)
    }

    pub async fn put_counter_async(&self, cache_id: &str, key: &str, value: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}", self.base_url, cache_id);
        let req = PutCounterRequest { key: key.to_string(), value };
        let resp = self.async_client.post(&url).json(&req).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<PutItemResponse>().await?)
    }

    pub async fn put_timed_counter_async(&self, cache_id: &str, key: &str, value: i64, ttl: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/timed/{}", self.base_url, cache_id);
        let req = PutTimedCounterRequest { key: key.to_string(), value, ttl };
        let resp = self.async_client.post(&url).json(&req).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<PutItemResponse>().await?)
    }

    pub async fn get_counter_async(&self, cache_id: &str, key: &str) -> Result<CounterResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}/{}", self.base_url, cache_id, key);
        let resp = self.async_client.get(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<CounterResponse>().await?)
    }

    pub async fn delete_counter_async(&self, cache_id: &str, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}/{}", self.base_url, cache_id, key);
        let resp = self.async_client.delete(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<DeleteItemResponse>().await?)
    }

    pub async fn delete_counter_cache_async(&self, cache_id: &str) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}", self.base_url, cache_id);
        let resp = self.async_client.delete(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<DeleteCacheResponse>().await?)
    }

    pub async fn increment_counter_async(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.counter_amount_op_async("increment", cache_id, key, amount).await
    }

    pub async fn decrement_counter_async(&self, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        self.counter_amount_op_async("decrement", cache_id, key, amount).await
    }

    async fn counter_amount_op_async(&self, op: &str, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}/{}", self.base_url, op, cache_id);
        let req = CounterAmountRequest { key: key.to_string(), amount };
        let resp = self.async_client.post(&url).json(&req).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<CounterResponse>().await?)
    }

    pub async fn set_counter_async(&self, cache_id: &str, key: &str, value: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/set/{}", self.base_url, cache_id);
        let req = PutCounterRequest { key: key.to_string(), value };
        let resp = self.async_client.post(&url).json(&req).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<CounterResponse>().await?)
    }

    // --- Additional Counter Operations (Sync) ---

    pub fn get_counter_items(&self, cache_id: &str) -> Result<GetCountersResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}", self.base_url, cache_id);
        let resp = self.get_sync_client().get(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<GetCountersResponse>()?)
    }

    pub fn clear_counter_cache(&self, cache_id: &str) -> Result<ClearCacheResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}", self.base_url, cache_id);
        let resp = self.get_sync_client().patch(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<ClearCacheResponse>()?)
    }

    pub fn cancel_counter_item_removal(&self, cache_id: &str, key: &str) -> Result<CancelItemRemovalResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}/{}/cancel-removal", self.base_url, cache_id, key);
        let resp = self.get_sync_client().post(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<CancelItemRemovalResponse>()?)
    }

    pub fn get_counter_caches(&self) -> Result<Vec<CacheDetails>, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters-caches", self.base_url);
        let resp = self.get_sync_client().get(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<Vec<CacheDetails>>()?)
    }

    pub fn get_counter_stats(&self) -> Result<CacheStatsResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters-stats", self.base_url);
        let resp = self.get_sync_client().get(&url).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<CacheStatsResponse>()?)
    }

    // --- Additional Counter Operations (Async) ---

    pub async fn get_counter_items_async(&self, cache_id: &str) -> Result<GetCountersResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}", self.base_url, cache_id);
        let resp = self.async_client.get(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<GetCountersResponse>().await?)
    }

    pub async fn clear_counter_cache_async(&self, cache_id: &str) -> Result<ClearCacheResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}", self.base_url, cache_id);
        let resp = self.async_client.patch(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<ClearCacheResponse>().await?)
    }

    pub async fn cancel_counter_item_removal_async(&self, cache_id: &str, key: &str) -> Result<CancelItemRemovalResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}/{}/cancel-removal", self.base_url, cache_id, key);
        let resp = self.async_client.post(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<CancelItemRemovalResponse>().await?)
    }

    pub async fn get_counter_caches_async(&self) -> Result<Vec<CacheDetails>, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters-caches", self.base_url);
        let resp = self.async_client.get(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<Vec<CacheDetails>>().await?)
    }

    pub async fn get_counter_stats_async(&self) -> Result<CacheStatsResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters-stats", self.base_url);
        let resp = self.async_client.get(&url).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<CacheStatsResponse>().await?)
    }

    pub fn get_counter_cache(&self, cache_id: &str) -> EmbeddedCounterCache<'_> {
        EmbeddedCounterCache::new(self, cache_id.to_string())
    }

    // --- WebSocket ---
    // Note: Rust websocket handling is often done in a loop in main application.
    // This helper connects and returns the stream.

    pub fn subscribe(&self, cache_ids: &str) -> Result<tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>, Box<dyn Error>> {
        self.subscribe_ext(cache_ids, false)
    }

    pub fn subscribe_ext(&self, cache_ids: &str, hydrate: bool) -> Result<tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>, Box<dyn Error>> {
        self.subscribe_filtered(cache_ids, hydrate, None, None)
    }

    /// Subscribe to one or more caches with optional key filters and subscription mode.
    ///
    /// * `keys` — optional comma-separated key filter(s); each token is `cacheId:key` or a bare
    ///   `key`. When set, the subscription is restricted to those keys.
    /// * `mode` — optional subscription mode, `"full"` (default, emits `ADD_ITEM` events) or
    ///   `"patch"` (emits `PATCH_ITEM` events carrying only changed fields). Cache-only.
    pub fn subscribe_filtered(&self, cache_ids: &str, hydrate: bool, keys: Option<&str>, mode: Option<&str>) -> Result<tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>, Box<dyn Error>> {
        let prefix = if hydrate {
            if cache_ids.contains(',') { "/api/ws/v1/caches/hydrate" } else { "/api/ws/v1/cache/hydrate" }
        } else {
            if cache_ids.contains(',') { "/api/ws/v1/caches" } else { "/api/ws/v1/cache" }
        };

        let url = format!("{}/{}", self.ws_url.trim_end_matches('/'), prefix.trim_start_matches('/'));
        let final_url = format!("{}/{}{}", url, cache_ids, ws_query(keys, mode));
        let (socket, _) = connect(Url::parse(&final_url)?)?;
        Ok(socket)
    }

    pub fn subscribe_counter(&self, cache_ids: &str) -> Result<tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>, Box<dyn Error>> {
        self.subscribe_counter_ext(cache_ids, false)
    }

    pub fn subscribe_counter_ext(&self, cache_ids: &str, hydrate: bool) -> Result<tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>, Box<dyn Error>> {
        self.subscribe_counter_filtered(cache_ids, hydrate, None)
    }

    /// Subscribe to one or more counter caches with optional key filters.
    ///
    /// * `keys` — optional comma-separated key filter(s); each token is `cacheId:key` or a bare
    ///   `key`. (Patch `mode` is cache-only and is not supported for counter subscriptions.)
    pub fn subscribe_counter_filtered(&self, cache_ids: &str, hydrate: bool, keys: Option<&str>) -> Result<tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>, Box<dyn Error>> {
        let prefix = if hydrate {
            if cache_ids.contains(',') { "/api/ws/v1/counters/hydrate" } else { "/api/ws/v1/counter/hydrate" }
        } else {
            if cache_ids.contains(',') { "/api/ws/v1/counters" } else { "/api/ws/v1/counter" }
        };

        let url = format!("{}/{}", self.ws_url.trim_end_matches('/'), prefix.trim_start_matches('/'));
        let final_url = format!("{}/{}{}", url, cache_ids, ws_query(keys, None));
        let (socket, _) = connect(Url::parse(&final_url)?)?;
        Ok(socket)
    }
}

/// Build the optional WebSocket query string for key filters and subscription mode.
///
/// Returns `""` when neither is set, otherwise `"?keys=...&mode=..."` with values properly
/// percent-encoded (e.g. `:` -> `%3A`, `,` -> `%2C`). Mirrors the Python client's `_ws_query`.
pub(crate) fn ws_query(keys: Option<&str>, mode: Option<&str>) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    let mut any = false;
    if let Some(k) = keys {
        serializer.append_pair("keys", k);
        any = true;
    }
    if let Some(m) = mode {
        serializer.append_pair("mode", m);
        any = true;
    }
    if any {
        format!("?{}", serializer.finish())
    } else {
        String::new()
    }
}

#[cfg(test)]
mod ws_query_tests {
    use super::ws_query;

    #[test]
    fn test_ws_query_neither() {
        assert_eq!(ws_query(None, None), "");
    }

    #[test]
    fn test_ws_query_keys_only() {
        assert_eq!(ws_query(Some("key1"), None), "?keys=key1");
    }

    #[test]
    fn test_ws_query_mode_only() {
        assert_eq!(ws_query(None, Some("patch")), "?mode=patch");
    }

    #[test]
    fn test_ws_query_both() {
        assert_eq!(ws_query(Some("key1"), Some("patch")), "?keys=key1&mode=patch");
    }

    #[test]
    fn test_ws_query_encodes_colon_and_comma() {
        // cacheId:key tokens and comma separators must be percent-encoded.
        assert_eq!(ws_query(Some("c1:key1,c1:key2"), None), "?keys=c1%3Akey1%2Cc1%3Akey2");
    }
}
