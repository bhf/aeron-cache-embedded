use reqwest::{Client as ReqwestClient, blocking::Client as BlockingClient};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tungstenite::{connect};
use url::Url;

mod embedded_cache;
pub use embedded_cache::EmbeddedAeronCache;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateRequest {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PutItemRequest {
    #[serde(rename = "cacheId")]
    pub cache_id: String,
    pub key: String,
    pub value: String,
}

pub struct AeronCacheClient {
    base_url: String,
    ws_url: String,
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

    pub fn create_cache(&self, cache_id: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache", self.base_url);
        let req = CreateRequest { cache_id: cache_id.to_string() };
        let resp = self.get_sync_client().post(&url)
            .json(&req)
            .send()?;
        let text = resp.text()?;
        Ok(text)
    }

    pub fn put_item(&self, cache_id: &str, key: &str, value: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}", self.base_url, cache_id);
        let req = PutItemRequest { cache_id: cache_id.to_string(), key: key.to_string(), value: value.to_string() };
        let resp = self.get_sync_client().post(&url)
            .json(&req)
            .send()?;
        let text = resp.text()?;
        Ok(text)
    }

    pub fn get_item(&self, cache_id: &str, key: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}/{}", self.base_url, cache_id, key);
        let resp = self.get_sync_client().get(&url).send()?;
        let text = resp.text()?;
        Ok(text)
    }

    pub fn delete_item(&self, cache_id: &str, key: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}/{}", self.base_url, cache_id, key);
        let resp = self.get_sync_client().delete(&url).send()?;
        let text = resp.text()?;
        Ok(text)
    }
    
    pub fn delete_cache(&self, cache_id: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}", self.base_url, cache_id);
        let resp = self.get_sync_client().delete(&url).send()?;
        let text = resp.text()?;
        Ok(text)
    }

    // --- Async Operations ---

    pub async fn create_cache_async(&self, cache_id: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache", self.base_url);
        let req = CreateRequest { cache_id: cache_id.to_string() };
        let resp = self.async_client.post(&url)
            .json(&req)
            .send()
            .await?;
        let text = resp.text().await?;
        Ok(text)
    }

    pub async fn put_item_async(&self, cache_id: &str, key: &str, value: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}", self.base_url, cache_id);
        let req = PutItemRequest { cache_id: cache_id.to_string(), key: key.to_string(), value: value.to_string() };
        let resp = self.async_client.post(&url)
            .json(&req)
            .send()
            .await?;
        let text = resp.text().await?;
        Ok(text)
    }

    pub async fn get_item_async(&self, cache_id: &str, key: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}/{}", self.base_url, cache_id, key);
        let resp = self.async_client.get(&url).send().await?;
        let text = resp.text().await?;
        Ok(text)
    }

    pub async fn delete_item_async(&self, cache_id: &str, key: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}/{}", self.base_url, cache_id, key);
        let resp = self.async_client.delete(&url).send().await?;
        let text = resp.text().await?;
        Ok(text)
    }
    
    pub async fn delete_cache_async(&self, cache_id: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/api/v1/cache/{}", self.base_url, cache_id);
        let resp = self.async_client.delete(&url).send().await?;
        let text = resp.text().await?;
        Ok(text)
    }

    pub fn get_cache(&self, cache_id: &str) -> EmbeddedAeronCache {
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
