use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

/// Represents the role of a connected relay server
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayRole {
    /// Primary/Home relay: Can receive and process new BIND_REQ contracts.
    Primary,
    /// Secondary/Guest relay: Used only for routing existing contracts. Rejects new contracts.
    Secondary,
}

#[derive(Debug, Clone)]
pub struct RelayConfig {
    pub url: String,
    pub role: RelayRole,
}

/// The state of a single relay connection
struct RelayConnection {
    #[allow(dead_code)]
    config: RelayConfig,
    tx: mpsc::Sender<Vec<u8>>,
    // We could store connection status (Connected, Backoff) here
}

/// Manages a pool of WebSocket connections to multiple relays.
pub struct RelayManager {
    connections: Arc<RwLock<HashMap<String, RelayConnection>>>,
    // In a real app, we'd also have an event channel for incoming messages going UP to the application
}

impl Default for RelayManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RelayManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a new relay to the connection pool and start connecting
    pub async fn add_relay(&self, url: &str, role: RelayRole) -> Result<()> {
        let mut conns = self.connections.write().await;
        if conns.contains_key(url) {
            return Ok(()); // Already configured
        }

        let (tx, rx) = mpsc::channel::<Vec<u8>>(100);

        let config = RelayConfig {
            url: url.to_string(),
            role,
        };

        conns.insert(
            url.to_string(),
            RelayConnection {
                config: config.clone(),
                tx,
            },
        );

        // Spawn connection task
        let url_clone = url.to_string();
        tokio::spawn(async move {
            Self::connection_loop(url_clone, config.role, rx).await;
        });

        Ok(())
    }

    /// Remove a relay, dropping its sender channel which will terminate the connection loop
    pub async fn remove_relay(&self, url: &str) {
        let mut conns = self.connections.write().await;
        conns.remove(url);
    }

    /// Send a message to ALL connected relays (e.g., for a broad presence broadcast)
    pub async fn broadcast(&self, payload: Vec<u8>) {
        let conns = self.connections.read().await;
        for conn in conns.values() {
            let _ = conn.tx.send(payload.clone()).await;
        }
    }

    /// Send a message to specific relays based on a routing profile
    pub async fn send_to_relays(&self, target_urls: &[String], payload: Vec<u8>) {
        let conns = self.connections.read().await;
        for url in target_urls {
            if let Some(conn) = conns.get(url) {
                let _ = conn.tx.send(payload.clone()).await;
            }
        }
    }

    /// Main loop for maintaining a single WebSocket connection
    async fn connection_loop(url_str: String, _role: RelayRole, mut rx: mpsc::Receiver<Vec<u8>>) {
        loop {
            // Reconnection delay could be added here
            let url = match Url::parse(&url_str) {
                Ok(u) => u,
                Err(_) => return, // Invalid URL, terminate task
            };

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    let (mut write, mut read) = ws_stream.split();

                    loop {
                        tokio::select! {
                            // Check for outgoing messages
                            msg = rx.recv() => {
                                match msg {
                                    Some(data) => {
                                        if write.send(Message::Binary(data)).await.is_err() {
                                            break; // Connection dropped
                                        }
                                    }
                                    None => return, // Channel closed, manager removed this relay
                                }
                            }
                            // Check for incoming messages
                            msg = read.next() => {
                                match msg {
                                    Some(Ok(Message::Binary(_data))) => {
                                        // TODO: Route incoming data up to the ContractManager
                                        // Enforce "Guest Mode" drops on Secondary relays here
                                    }
                                    Some(Ok(Message::Close(_))) | None => {
                                        break; // Disconnected
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(_e) => {
                    // Connection failed, wait and retry
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    }
}
