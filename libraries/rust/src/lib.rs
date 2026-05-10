use reqwest::{Client as ReqwestClient, blocking::Client as BlockingClient};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tungstenite::{connect};
use url::Url;

pub mod embedded_cache;
pub use embedded_cache::EmbeddedAeronCache;

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

    pub fn get_cache(&self, cache_id: &str) -> EmbeddedAeronCache<'_> {
        EmbeddedAeronCache::new(self, cache_id.to_string())
    }

    // --- WebSocket ---
    // Note: Rust websocket handling is often done in a loop in main application.
    // This helper connects and returns the stream.
    
    pub fn subscribe(&self, cache_id: &str) -> Result<tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>, Box<dyn Error>> {
        let url = format!("{}/api/ws/v1/cache/{}", self.ws_url, cache_id);
        let (socket, _) = connect(Url::parse(&url)?)?;
        Ok(socket)
    }
}
