use reqwest::{Client as ReqwestClient, blocking::Client as BlockingClient};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tungstenite::{connect};
use url::Url;

pub mod embedded_cache;
pub use embedded_cache::EmbeddedAeronCache;

pub mod embedded_counter_cache;
pub use embedded_counter_cache::EmbeddedCounterCache;

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
    pub operation_status: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PutItemRequest {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PutItemResponse {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    pub key: String,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(rename = "operationStatus")]
    pub operation_status: Option<String>,
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
    pub operation_status: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteItemResponse {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    pub key: String,
    #[serde(rename = "operationStatus")]
    pub operation_status: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteCacheResponse {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    #[serde(rename = "operationStatus")]
    pub operation_status: Option<String>,
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
pub struct CounterResponse {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    pub key: String,
    #[serde(default)]
    pub value: i64,
    #[serde(rename = "operationStatus")]
    pub operation_status: Option<String>,
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

// --- Bulk operations ---

/// String constants for the `operationType` field of a bulk operation.
pub mod bulk_operation_type {
    pub const NONE: &str = "NONE";
    pub const CREATE_CACHE: &str = "CREATE_CACHE";
    pub const ADD_ITEM: &str = "ADD_ITEM";
    pub const REMOVE_ITEM: &str = "REMOVE_ITEM";
    pub const CLEAR_CACHE: &str = "CLEAR_CACHE";
    pub const GET_ITEM: &str = "GET_ITEM";
    pub const DELETE_CACHE: &str = "DELETE_CACHE";
    pub const CREATE_COUNTER_CACHE: &str = "CREATE_COUNTER_CACHE";
    pub const ADD_COUNTER: &str = "ADD_COUNTER";
    pub const REMOVE_COUNTER: &str = "REMOVE_COUNTER";
    pub const CLEAR_COUNTER_CACHE: &str = "CLEAR_COUNTER_CACHE";
    pub const GET_COUNTER: &str = "GET_COUNTER";
    pub const DELETE_COUNTER_CACHE: &str = "DELETE_COUNTER_CACHE";
    pub const INCREMENT_COUNTER: &str = "INCREMENT_COUNTER";
    pub const DECREMENT_COUNTER: &str = "DECREMENT_COUNTER";
    pub const SET_COUNTER: &str = "SET_COUNTER";
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CacheOperationRequest {
    #[serde(rename = "operationType")]
    pub operation_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i64>,
    #[serde(rename = "counterValue", skip_serializing_if = "Option::is_none")]
    pub counter_value: Option<i64>,
    #[serde(rename = "requestId", skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(rename = "cacheId", skip_serializing_if = "Option::is_none")]
    pub cache_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BulkCacheOpsRequest {
    #[serde(rename = "requestId", skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub operations: Vec<CacheOperationRequest>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CacheOperationResponse {
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "cacheId")]
    pub cache_id: Option<String>,
    pub key: Option<String>,
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BulkCacheOpsResponse {
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    #[serde(rename = "operationResponses", default)]
    pub operation_responses: Vec<CacheOperationResponse>,
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
        let req = PutItemRequest { cache_id: cache_id.to_string(), key: key.to_string(), value: value.to_string() };
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
        let req = PutItemRequest { cache_id: cache_id.to_string(), key: key.to_string(), value: value.to_string() };
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

    // --- Counter Operations (Sync) ---

    pub fn create_counter_cache(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/", self.base_url);
        let resp = self.get_sync_client().post(&url)
            .json(&serde_json::json!({ "cacheId": cache_id }))
            .send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<CreateResponse>()?)
    }

    pub fn put_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}", self.base_url, cache_id);
        let resp = self.get_sync_client().post(&url)
            .json(&serde_json::json!({ "key": key, "value": value }))
            .send()?;
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

    pub fn set_counter(&self, cache_id: &str, key: &str, value: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/set/{}", self.base_url, cache_id);
        let resp = self.get_sync_client().post(&url)
            .json(&serde_json::json!({ "key": key, "value": value }))
            .send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<CounterResponse>()?)
    }

    fn counter_amount_op(&self, op: &str, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}/{}", self.base_url, op, cache_id);
        let resp = self.get_sync_client().post(&url)
            .json(&serde_json::json!({ "key": key, "amount": amount }))
            .send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<CounterResponse>()?)
    }

    pub fn bulk_ops(&self, request: &BulkCacheOpsRequest) -> Result<BulkCacheOpsResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/bulkops", self.base_url);
        let resp = self.get_sync_client().post(&url).json(request).send()?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text()?).into());
        }
        Ok(resp.json::<BulkCacheOpsResponse>()?)
    }

    // --- Counter Operations (Async) ---

    pub async fn create_counter_cache_async(&self, cache_id: &str) -> Result<CreateResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/", self.base_url);
        let resp = self.async_client.post(&url)
            .json(&serde_json::json!({ "cacheId": cache_id }))
            .send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<CreateResponse>().await?)
    }

    pub async fn put_counter_async(&self, cache_id: &str, key: &str, value: i64) -> Result<PutItemResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}", self.base_url, cache_id);
        let resp = self.async_client.post(&url)
            .json(&serde_json::json!({ "key": key, "value": value }))
            .send().await?;
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

    pub async fn set_counter_async(&self, cache_id: &str, key: &str, value: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/set/{}", self.base_url, cache_id);
        let resp = self.async_client.post(&url)
            .json(&serde_json::json!({ "key": key, "value": value }))
            .send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<CounterResponse>().await?)
    }

    async fn counter_amount_op_async(&self, op: &str, cache_id: &str, key: &str, amount: i64) -> Result<CounterResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/counters/{}/{}", self.base_url, op, cache_id);
        let resp = self.async_client.post(&url)
            .json(&serde_json::json!({ "key": key, "amount": amount }))
            .send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<CounterResponse>().await?)
    }

    pub async fn bulk_ops_async(&self, request: &BulkCacheOpsRequest) -> Result<BulkCacheOpsResponse, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/bulkops", self.base_url);
        let resp = self.async_client.post(&url).json(request).send().await?;
        if !resp.status().is_success() && resp.status() != 400 && resp.status() != 401 && resp.status() != 404 {
            return Err(format!("HTTP Error: {} - {}", resp.status(), resp.text().await?).into());
        }
        Ok(resp.json::<BulkCacheOpsResponse>().await?)
    }

    pub fn get_cache(&self, cache_id: &str) -> EmbeddedAeronCache<'_> {
        EmbeddedAeronCache::new(self, cache_id.to_string())
    }

    pub fn get_counter_cache(&self, cache_id: &str) -> EmbeddedCounterCache<'_> {
        EmbeddedCounterCache::new(self, cache_id.to_string())
    }

    // --- WebSocket ---
    // Note: Rust websocket handling is often done in a loop in main application.
    // This helper connects and returns the stream.
    
    pub fn subscribe(&self, cache_id: &str) -> Result<tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>, Box<dyn Error>> {
        let url = format!("{}/api/ws/v1/cache/{}", self.ws_url, cache_id);
        let (socket, _) = connect(Url::parse(&url)?)?;
        Ok(socket)
    }

    pub fn subscribe_counter(&self, cache_id: &str) -> Result<tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>, Box<dyn Error>> {
        let url = format!("{}/api/ws/v1/counter/{}", self.ws_url, cache_id);
        let (socket, _) = connect(Url::parse(&url)?)?;
        Ok(socket)
    }
}
