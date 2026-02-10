// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! WebSocket Handler for bidirectional real-time communication
//!
//! This module provides WebSocket-based communication for scenarios requiring
//! bidirectional messaging, such as live chat streaming, agent control, and
//! collaborative editing.
//!
//! Note: For simpler use cases, the SSE-based sync (in sync.rs) may be preferred
//! as it has better HTTP/2 compatibility and doesn't require connection upgrades.

use actix_web::{web, Error, HttpRequest, HttpResponse};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use uuid::Uuid;

// =============================================================================
// WebSocket Configuration
// =============================================================================

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(60);

// =============================================================================
// WebSocket Message Types
// =============================================================================

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsClientMessage {
    /// Subscribe to a channel
    Subscribe { channel: String },
    /// Unsubscribe from a channel
    Unsubscribe { channel: String },

    /// Start streaming response for a session
    StreamStart { session_id: String, model: String },
    /// Cancel streaming for a session
    StreamCancel { session_id: String },
    /// Send input during streaming
    StreamInput { session_id: String, content: String },

    /// Send command to an agent
    AgentCommand {
        agent_id: String,
        command: String,
        params: Option<serde_json::Value>,
    },

    /// Request sync delta from version
    SyncRequest { from_version: u64 },

    /// Ping message for keepalive
    Ping { timestamp: i64 },
}

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsServerMessage {
    /// Connection established
    Connected { client_id: String, version: u64 },
    /// Error occurred
    Error { code: String, message: String },

    /// Successfully subscribed to channel
    Subscribed { channel: String },
    /// Successfully unsubscribed from channel
    Unsubscribed { channel: String },

    /// Streaming token received
    StreamToken { session_id: String, token: String },
    /// Streaming completed
    StreamComplete {
        session_id: String,
        message_id: String,
    },
    /// Streaming error occurred
    StreamError { session_id: String, error: String },

    /// Agent event received
    AgentEvent {
        agent_id: String,
        event: String,
        data: Option<serde_json::Value>,
    },

    /// Sync event for real-time updates
    SyncEvent {
        entity_type: String,
        entity_id: String,
        operation: String,
        data: Option<serde_json::Value>,
        version: u64,
    },

    /// Pong response to ping
    Pong { timestamp: i64 },
}

// =============================================================================
// WebSocket State Management
// =============================================================================

/// Information about a connected client
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub id: String,
    pub connected_at: Instant,
    pub last_heartbeat: Instant,
    pub subscriptions: Vec<String>,
}

/// Global WebSocket state shared across connections
pub struct WebSocketState {
    /// Broadcast channel for server-wide messages
    pub broadcast_tx: broadcast::Sender<WsServerMessage>,
    /// Per-channel broadcast senders
    pub channel_senders: RwLock<HashMap<String, broadcast::Sender<WsServerMessage>>>,
    /// Connected clients info
    pub clients: RwLock<HashMap<String, ClientInfo>>,
    /// Current sync version
    pub version: std::sync::atomic::AtomicU64,
}

impl WebSocketState {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1024);
        Self {
            broadcast_tx,
            channel_senders: RwLock::new(HashMap::new()),
            clients: RwLock::new(HashMap::new()),
            version: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Get or create a channel sender
    pub fn get_channel_sender(&self, channel: &str) -> broadcast::Sender<WsServerMessage> {
        {
            let channels = self.channel_senders.read().unwrap();
            if let Some(sender) = channels.get(channel) {
                return sender.clone();
            }
        }

        let mut channels = self.channel_senders.write().unwrap();
        let entry = channels
            .entry(channel.to_string())
            .or_insert_with(|| broadcast::channel(256).0);
        entry.clone()
    }

    /// Broadcast to all clients
    pub fn broadcast(&self, msg: WsServerMessage) {
        let _ = self.broadcast_tx.send(msg);
    }

    /// Broadcast to a specific channel
    pub fn broadcast_to_channel(&self, channel: &str, msg: WsServerMessage) {
        let channels = self.channel_senders.read().unwrap();
        if let Some(sender) = channels.get(channel) {
            let _ = sender.send(msg);
        }
    }

    /// Increment version and return new value
    pub fn increment_version(&self) -> u64 {
        self.version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1
    }

    /// Get current version
    pub fn current_version(&self) -> u64 {
        self.version.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Register a new client
    pub fn register_client(&self, id: &str) {
        let mut clients = self.clients.write().unwrap();
        clients.insert(
            id.to_string(),
            ClientInfo {
                id: id.to_string(),
                connected_at: Instant::now(),
                last_heartbeat: Instant::now(),
                subscriptions: Vec::new(),
            },
        );
    }

    /// Unregister a client
    pub fn unregister_client(&self, id: &str) {
        let mut clients = self.clients.write().unwrap();
        clients.remove(id);
    }

    /// Get client count
    pub fn client_count(&self) -> usize {
        self.clients.read().unwrap().len()
    }
}

impl Default for WebSocketState {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// WebSocket Handler
// =============================================================================

/// Handle incoming WebSocket message
fn handle_client_message(
    client_id: &str,
    msg: WsClientMessage,
    state: &WebSocketState,
) -> Option<WsServerMessage> {
    match msg {
        WsClientMessage::Subscribe { channel } => {
            // Update client subscriptions
            if let Ok(mut clients) = state.clients.write() {
                if let Some(client) = clients.get_mut(client_id) {
                    if !client.subscriptions.contains(&channel) {
                        client.subscriptions.push(channel.clone());
                    }
                }
            }
            Some(WsServerMessage::Subscribed { channel })
        }

        WsClientMessage::Unsubscribe { channel } => {
            // Update client subscriptions
            if let Ok(mut clients) = state.clients.write() {
                if let Some(client) = clients.get_mut(client_id) {
                    client.subscriptions.retain(|c| c != &channel);
                }
            }
            Some(WsServerMessage::Unsubscribed { channel })
        }

        WsClientMessage::Ping { timestamp } => Some(WsServerMessage::Pong { timestamp }),

        WsClientMessage::StreamStart { session_id, model } => {
            log::info!(
                "Client {} requested stream start for {} with model {}",
                client_id,
                session_id,
                model
            );
            // TODO: Implement streaming start
            None
        }

        WsClientMessage::StreamCancel { session_id } => {
            log::info!(
                "Client {} requested stream cancel for {}",
                client_id,
                session_id
            );
            // TODO: Implement streaming cancel
            None
        }

        WsClientMessage::StreamInput {
            session_id,
            content,
        } => {
            log::info!(
                "Client {} sent input for {}: {} bytes",
                client_id,
                session_id,
                content.len()
            );
            // TODO: Implement streaming input
            None
        }

        WsClientMessage::AgentCommand {
            agent_id,
            command,
            params,
        } => {
            log::info!(
                "Client {} sent agent command {} to {}: {:?}",
                client_id,
                command,
                agent_id,
                params
            );
            // TODO: Implement agent commands
            None
        }

        WsClientMessage::SyncRequest { from_version } => {
            log::info!(
                "Client {} requested sync from version {}",
                client_id,
                from_version
            );
            // TODO: Implement sync delta response
            None
        }
    }
}

/// WebSocket endpoint handler using actix-ws
pub async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    state: web::Data<WebSocketState>,
) -> Result<HttpResponse, Error> {
    // Perform WebSocket handshake
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    let client_id = Uuid::new_v4().to_string();
    let state_clone = state.clone();

    // Register client
    state.register_client(&client_id);

    // Send connected message
    let connected_msg = WsServerMessage::Connected {
        client_id: client_id.clone(),
        version: state.current_version(),
    };
    if let Ok(json) = serde_json::to_string(&connected_msg) {
        let _ = session.text(json).await;
    }

    log::info!("WebSocket client {} connected", client_id);

    // Subscribe to broadcast channel
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    // Spawn handler task
    let client_id_clone = client_id.clone();
    actix_web::rt::spawn(async move {
        let mut heartbeat_interval = tokio::time::interval(HEARTBEAT_INTERVAL);
        let mut last_heartbeat = Instant::now();

        loop {
            tokio::select! {
                // Handle incoming messages
                Some(msg_result) = msg_stream.next() => {
                    match msg_result {
                        Ok(actix_ws::Message::Text(text)) => {
                            last_heartbeat = Instant::now();
                            if let Ok(client_msg) = serde_json::from_str::<WsClientMessage>(&text) {
                                if let Some(response) = handle_client_message(
                                    &client_id_clone,
                                    client_msg,
                                    &state_clone,
                                ) {
                                    if let Ok(json) = serde_json::to_string(&response) {
                                        let _ = session.text(json).await;
                                    }
                                }
                            } else {
                                let error_msg = WsServerMessage::Error {
                                    code: "invalid_message".to_string(),
                                    message: "Failed to parse message".to_string(),
                                };
                                if let Ok(json) = serde_json::to_string(&error_msg) {
                                    let _ = session.text(json).await;
                                }
                            }
                        }
                        Ok(actix_ws::Message::Ping(data)) => {
                            last_heartbeat = Instant::now();
                            let _ = session.pong(&data).await;
                        }
                        Ok(actix_ws::Message::Pong(_)) => {
                            last_heartbeat = Instant::now();
                        }
                        Ok(actix_ws::Message::Close(_)) => {
                            log::info!("WebSocket client {} requested close", client_id_clone);
                            break;
                        }
                        _ => {}
                    }
                }

                // Handle broadcast messages
                Ok(msg) = broadcast_rx.recv() => {
                    if let Ok(json) = serde_json::to_string(&msg) {
                        let _ = session.text(json).await;
                    }
                }

                // Heartbeat check
                _ = heartbeat_interval.tick() => {
                    if Instant::now().duration_since(last_heartbeat) > CLIENT_TIMEOUT {
                        log::warn!("WebSocket client {} timed out", client_id_clone);
                        break;
                    }
                    let _ = session.ping(b"").await;
                }
            }
        }

        // Cleanup
        state_clone.unregister_client(&client_id_clone);
        let _ = session.close(None).await;
        log::info!("WebSocket client {} disconnected", client_id_clone);
    });

    Ok(response)
}

/// Configure WebSocket routes
pub fn configure_websocket_routes(cfg: &mut web::ServiceConfig, state: web::Data<WebSocketState>) {
    cfg.app_data(state).route("/ws", web::get().to(ws_handler));
}

// =============================================================================
// Helper Functions for Broadcasting
// =============================================================================

/// Broadcast a sync event to all clients
pub fn broadcast_sync_event(
    state: &WebSocketState,
    entity_type: &str,
    entity_id: &str,
    operation: &str,
    data: Option<serde_json::Value>,
) {
    let version = state.increment_version();
    let msg = WsServerMessage::SyncEvent {
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        operation: operation.to_string(),
        data,
        version,
    };
    state.broadcast(msg);
}

/// Broadcast a stream token to a specific session channel
pub fn broadcast_stream_token(state: &WebSocketState, session_id: &str, token: &str) {
    let msg = WsServerMessage::StreamToken {
        session_id: session_id.to_string(),
        token: token.to_string(),
    };
    state.broadcast_to_channel(&format!("session:{}", session_id), msg);
}

/// Broadcast stream completion
pub fn broadcast_stream_complete(state: &WebSocketState, session_id: &str, message_id: &str) {
    let msg = WsServerMessage::StreamComplete {
        session_id: session_id.to_string(),
        message_id: message_id.to_string(),
    };
    state.broadcast_to_channel(&format!("session:{}", session_id), msg);
}

/// Broadcast an agent event
pub fn broadcast_agent_event(
    state: &WebSocketState,
    agent_id: &str,
    event: &str,
    data: Option<serde_json::Value>,
) {
    let msg = WsServerMessage::AgentEvent {
        agent_id: agent_id.to_string(),
        event: event.to_string(),
        data,
    };
    state.broadcast_to_channel(&format!("agent:{}", agent_id), msg);
}
