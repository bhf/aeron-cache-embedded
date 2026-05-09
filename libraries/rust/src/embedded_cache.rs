use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, RwLock};
use tungstenite::{Message};
use crate::{AeronCacheClient, CacheUpdateEvent, PutItemResponse, GetItemResponse, DeleteItemResponse, DeleteCacheResponse};

pub struct EmbeddedAeronCache<'a> {
    client: &'a AeronCacheClient,
    cache_id: String,
    local_cache: Arc<RwLock<HashMap<String, String>>>,
}

pub struct UpdatingWebSocket {
    socket: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    client_ws_url: String,
    cache_id: String,
    local_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl UpdatingWebSocket {
    pub fn read_message(&mut self) -> Result<Message, Box<dyn Error>> {
        loop {
            match self.socket.read_message() {
                Ok(msg) => {
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
                                        if let Some(ref k) = event.item_key {
                                            cache.remove(k);
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
                    return Ok(msg);
                }
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    let url = format!("{}/api/ws/v1/cache/{}", self.client_ws_url, self.cache_id);
                    if let Ok((socket, _)) = tungstenite::connect(url::Url::parse(&url)?) {
                        self.socket = socket;
                    }
                }
            }
        }
    }

    pub fn write_message(&mut self, msg: Message) -> Result<(), Box<dyn Error>> {
        self.socket.write_message(msg).map_err(|e| e.into())
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

    pub fn insert(&self, key: &str, value: &str) -> Result<PutItemResponse, Box<dyn Error>> {
        self.client.put_item(&self.cache_id, key, value)
    }

    pub fn get(&self, key: &str) -> Result<GetItemResponse, Box<dyn Error>> {
        self.client.get_item(&self.cache_id, key)
    }

    pub fn remove(&self, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        self.client.delete_item(&self.cache_id, key)
    }

    pub fn clear(&self) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        self.client.delete_cache(&self.cache_id)
    }

    pub async fn insert_async(&self, key: &str, value: &str) -> Result<PutItemResponse, Box<dyn Error>> {
        self.client.put_item_async(&self.cache_id, key, value).await
    }

    pub async fn get_async(&self, key: &str) -> Result<GetItemResponse, Box<dyn Error>> {
        self.client.get_item_async(&self.cache_id, key).await
    }

    pub async fn remove_async(&self, key: &str) -> Result<DeleteItemResponse, Box<dyn Error>> {
        self.client.delete_item_async(&self.cache_id, key).await
    }

    pub async fn clear_async(&self) -> Result<DeleteCacheResponse, Box<dyn Error>> {
        self.client.delete_cache_async(&self.cache_id).await
    }

    pub fn subscribe(&self) -> Result<UpdatingWebSocket, Box<dyn Error>> {
        let socket = self.client.subscribe(&self.cache_id)?;
        Ok(UpdatingWebSocket {
            socket,
            client_ws_url: self.client.ws_url.clone(),
            cache_id: self.cache_id.clone(),
            local_cache: self.local_cache.clone(),
        })
    }
}
