use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, RwLock};
use tungstenite::{Message};
use crate::AeronCacheClient;

#[derive(Deserialize)]
struct CacheUpdateEvent {
    #[serde(rename = "eventType")]
    event_type: String,
    #[serde(rename = "itemKey")]
    item_key: Option<String>,
    #[serde(rename = "itemValue")]
    item_value: Option<String>,
}

pub struct EmbeddedAeronCache<'a> {
    client: &'a AeronCacheClient,
    cache_id: String,
    local_cache: Arc<RwLock<HashMap<String, String>>>,
}

pub struct UpdatingWebSocket<S> {
    socket: tungstenite::WebSocket<S>,
    local_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl<S> UpdatingWebSocket<S>
where
    S: std::io::Read + std::io::Write,
{
    pub fn read_message(&mut self) -> Result<Message, tungstenite::Error> {
        let msg = self.socket.read_message()?;
        if let Message::Text(ref text) = msg {
            if let Ok(event) = serde_json::from_str::<CacheUpdateEvent>(text) {
                if let Ok(mut cache) = self.local_cache.write() {
                    match event.event_type.as_str() {
                        "ADD_ITEM" => {
                            if let (Some(k), Some(v)) = (event.item_key, event.item_value) {
                                cache.insert(k, v);
                            }
                        }
                        "REMOVE_ITEM" => {
                            if let Some(k) = event.item_key {
                                cache.remove(&k);
                            }
                        }
                        "CLEAR_CACHE" | "DELETE_CACHE" => {
                            cache.clear();
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(msg)
    }

    pub fn write_message(&mut self, msg: Message) -> Result<(), tungstenite::Error> {
        self.socket.write_message(msg)
    }
}

impl<'a> EmbeddedAeronCache<'a> {
    pub fn new(client: &'a AeronCacheClient, cache_id: String) -> Self {
        EmbeddedAeronCache {
            client,
            cache_id,
            local_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_local(&self, key: &str) -> Option<String> {
        if let Ok(cache) = self.local_cache.read() {
            cache.get(key).cloned()
        } else {
            None
        }
    }

    pub fn insert(&self, key: &str, value: &str) -> Result<String, Box<dyn Error>> {
        self.client.put_item(&self.cache_id, key, value)
    }

    pub fn get(&self, key: &str) -> Result<String, Box<dyn Error>> {
        self.client.get_item(&self.cache_id, key)
    }

    pub fn remove(&self, key: &str) -> Result<String, Box<dyn Error>> {
        self.client.delete_item(&self.cache_id, key)
    }

    pub fn clear(&self) -> Result<String, Box<dyn Error>> {
        self.client.delete_cache(&self.cache_id)
    }

    pub async fn insert_async(&self, key: &str, value: &str) -> Result<String, Box<dyn Error>> {
        self.client.put_item_async(&self.cache_id, key, value).await
    }

    pub async fn get_async(&self, key: &str) -> Result<String, Box<dyn Error>> {
        self.client.get_item_async(&self.cache_id, key).await
    }

    pub async fn remove_async(&self, key: &str) -> Result<String, Box<dyn Error>> {
        self.client.delete_item_async(&self.cache_id, key).await
    }

    pub async fn clear_async(&self) -> Result<String, Box<dyn Error>> {
        self.client.delete_cache_async(&self.cache_id).await
    }

    pub fn subscribe(&self) -> Result<UpdatingWebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>, Box<dyn Error>> {
        let socket = self.client.subscribe(&self.cache_id)?;
        Ok(UpdatingWebSocket {
            socket,
            local_cache: self.local_cache.clone(),
        })
    }
}
