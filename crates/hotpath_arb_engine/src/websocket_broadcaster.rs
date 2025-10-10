use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    Json,
};
use futures::{sink::SinkExt, stream::StreamExt};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::property_store::{
    get_property_store, PropertyChangeEvent, PropertySource, PropertyValue, ValidationError,
};

/// WebSocket client connection management
#[derive(Debug, Clone)]
pub struct WebSocketClient {
    pub id: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub subscriptions: Vec<String>, // Property keys this client wants to receive
    pub sender: Option<broadcast::Sender<PropertyBroadcastMessage>>,
}

/// Message types for WebSocket communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PropertyBroadcastMessage {
    PropertyChanged {
        property_key: String,
        old_value: PropertyValue,
        new_value: PropertyValue,
        source: PropertySource,
        timestamp: u64,
        change_id: String,
    },
    PropertyUpdate {
        property_key: String,
        value: PropertyValue,
        validated: bool,
        error: Option<String>,
    },
    PropertyList {
        properties: HashMap<String, PropertyValue>,
    },
    SystemHealth {
        status: String,
        message: String,
        timestamp: u64,
    },
    Error {
        message: String,
        property_key: Option<String>,
    },
}

/// Incoming WebSocket messages from UI clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum PropertyControlMessage {
    Subscribe {
        property_keys: Vec<String>,
    },
    Unsubscribe {
        property_keys: Vec<String>,
    },
    SetProperty {
        property_key: String,
        value: PropertyValue,
        applied_by: String,
    },
    GetProperty {
        property_key: String,
    },
    GetAllProperties,
    EmergencyStop {
        reason: String,
        applied_by: String,
    },
}

/// WebSocket broadcaster for real-time property updates
pub struct WebSocketBroadcaster {
    clients: Arc<RwLock<HashMap<String, WebSocketClient>>>,
    property_receiver: broadcast::Receiver<PropertyChangeEvent>,
    broadcast_sender: broadcast::Sender<PropertyBroadcastMessage>,
}

impl WebSocketBroadcaster {
    pub fn new() -> Self {
        let property_store = get_property_store();
        let property_receiver = property_store.subscribe();
        let (broadcast_sender, _) = broadcast::channel(1000);

        let broadcaster = Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            property_receiver,
            broadcast_sender,
        };

        broadcaster
    }

    /// Start the background task that listens for property changes and broadcasts them
    pub async fn start_property_broadcasting(&mut self) {
        let _clients = Arc::clone(&self.clients);
        let broadcast_sender = self.broadcast_sender.clone();

        tokio::spawn(async move {
            info!("🔄 WebSocket property broadcaster started");

            // Get a new receiver for this task
            let property_store = get_property_store();
            let mut property_receiver = property_store.subscribe();

            loop {
                match property_receiver.recv().await {
                    Ok(property_event) => {
                        debug!(
                            "📡 Broadcasting property change: {} -> {:?}",
                            property_event.property_key, property_event.new_value
                        );

                        let broadcast_message = PropertyBroadcastMessage::PropertyChanged {
                            property_key: property_event.property_key.clone(),
                            old_value: property_event.old_value,
                            new_value: property_event.new_value,
                            source: property_event.source,
                            timestamp: property_event.timestamp,
                            change_id: property_event.change_id,
                        };

                        // Broadcast to all connected clients
                        if let Err(e) = broadcast_sender.send(broadcast_message) {
                            warn!("Failed to broadcast property change: {}", e);
                        }

                        // Update metrics
                        metrics::counter!("websocket_property_broadcasts_total").increment(1);
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(
                            "⚠️ WebSocket broadcaster lagged, skipped {} messages",
                            skipped
                        );
                        metrics::counter!("websocket_broadcaster_lag_total").increment(skipped);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        error!("❌ Property change broadcast channel closed");
                        break;
                    }
                }
            }
        });
    }

    /// Handle new WebSocket connection
    pub async fn handle_websocket_connection(&self, websocket: WebSocket, client_id: String) {
        let clients = Arc::clone(&self.clients);
        let mut broadcast_receiver = self.broadcast_sender.subscribe();

        // Add client to the connection list
        {
            let mut clients_lock = clients.write().await;
            clients_lock.insert(
                client_id.clone(),
                WebSocketClient {
                    id: client_id.clone(),
                    connected_at: chrono::Utc::now(),
                    subscriptions: Vec::new(),
                    sender: None,
                },
            );
        }

        info!("🔌 WebSocket client connected: {}", client_id);
        metrics::counter!("websocket_connections_total").increment(1);

        let (ws_sender, mut ws_receiver) = websocket.split();
        // Wrap sender in Arc<Mutex<..>> so both tasks can send safely
        let ws_sender = Arc::new(tokio::sync::Mutex::new(ws_sender));

        // Send initial property list to new client
        let all_properties = get_property_store().get_all_properties().await;
        let initial_message = PropertyBroadcastMessage::PropertyList {
            properties: all_properties,
        };

        if let Ok(msg_json) = serde_json::to_string(&initial_message) {
            if let Err(e) = ws_sender.lock().await.send(Message::Text(msg_json)).await {
                warn!(
                    "Failed to send initial property list to client {}: {}",
                    client_id, e
                );
            }
        }

        // Spawn task to handle outgoing messages (property broadcasts)
        let client_id_out = client_id.clone();
        let clients_out = Arc::clone(&clients);
        let ws_sender_out = Arc::clone(&ws_sender);
        let outgoing_task = tokio::spawn(async move {
            while let Ok(broadcast_msg) = broadcast_receiver.recv().await {
                // Check if client is still connected and subscribed to this property
                let should_send = {
                    let clients_lock = clients_out.read().await;
                    if let Some(client) = clients_lock.get(&client_id_out) {
                        match &broadcast_msg {
                            PropertyBroadcastMessage::PropertyChanged { property_key, .. } => {
                                client.subscriptions.is_empty() || // Send to all if no specific subscriptions
                                client.subscriptions.contains(property_key)
                            }
                            _ => true, // Send all other message types
                        }
                    } else {
                        false // Client disconnected
                    }
                };

                if should_send {
                    if let Ok(msg_json) = serde_json::to_string(&broadcast_msg) {
                        if let Err(e) = ws_sender_out
                            .lock()
                            .await
                            .send(Message::Text(msg_json))
                            .await
                        {
                            debug!("WebSocket send failed for client {}: {}", client_id_out, e);
                            break;
                        }
                    }
                }
            }
        });

        // Handle incoming messages (property updates, subscriptions)
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<PropertyControlMessage>(&text) {
                        Ok(control_msg) => {
                            if let Err(e) =
                                self.handle_control_message(control_msg, &client_id).await
                            {
                                warn!("Error handling control message from {}: {}", client_id, e);

                                let error_response = PropertyBroadcastMessage::Error {
                                    message: format!("Failed to process message: {}", e),
                                    property_key: None,
                                };

                                if let Ok(error_json) = serde_json::to_string(&error_response) {
                                    let _ = ws_sender
                                        .lock()
                                        .await
                                        .send(Message::Text(error_json))
                                        .await;
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Invalid JSON message from client {}: {}", client_id, e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("🔌 WebSocket client {} disconnected", client_id);
                    break;
                }
                Ok(_) => {
                    // Ignore binary and other message types
                }
                Err(e) => {
                    warn!("WebSocket error for client {}: {}", client_id, e);
                    break;
                }
            }
        }

        // Clean up client connection
        outgoing_task.abort();
        {
            let mut clients_lock = clients.write().await;
            clients_lock.remove(&client_id);
        }

        info!("🔌 WebSocket client {} cleaned up", client_id);
        metrics::counter!("websocket_disconnections_total").increment(1);
    }

    /// Handle control messages from WebSocket clients
    async fn handle_control_message(
        &self,
        message: PropertyControlMessage,
        client_id: &str,
    ) -> Result<()> {
        match message {
            PropertyControlMessage::Subscribe { property_keys } => {
                let mut clients = self.clients.write().await;
                if let Some(client) = clients.get_mut(client_id) {
                    client.subscriptions.extend(property_keys.clone());
                    client.subscriptions.sort();
                    client.subscriptions.dedup();

                    info!(
                        "📡 Client {} subscribed to properties: {:?}",
                        client_id, property_keys
                    );
                }
            }

            PropertyControlMessage::Unsubscribe { property_keys } => {
                let mut clients = self.clients.write().await;
                if let Some(client) = clients.get_mut(client_id) {
                    for key in property_keys {
                        client.subscriptions.retain(|k| k != &key);
                    }

                    info!("📡 Client {} unsubscribed from properties", client_id);
                }
            }

            PropertyControlMessage::SetProperty {
                property_key,
                value,
                applied_by,
            } => {
                info!(
                    "🎛️ Client {} attempting to set property: {} = {:?}",
                    client_id, property_key, value
                );

                // Validate and set property through PropertyStore
                let property_store = get_property_store();
                let result = property_store
                    .set_property_value(&property_key, value.clone(), PropertySource::HumanUI)
                    .await;

                // Send response back to client
                let response = match result {
                    Ok(_) => PropertyBroadcastMessage::PropertyUpdate {
                        property_key: property_key.clone(),
                        value,
                        validated: true,
                        error: None,
                    },
                    Err(e) => PropertyBroadcastMessage::PropertyUpdate {
                        property_key: property_key.clone(),
                        value,
                        validated: false,
                        error: Some(e.to_string()),
                    },
                };

                // Send response directly to requesting client
                if let Err(e) = self.broadcast_sender.send(response) {
                    warn!("Failed to send property update response: {}", e);
                }

                metrics::counter!("websocket_property_updates_total").increment(1);
            }

            PropertyControlMessage::GetProperty { property_key } => {
                // Get current property value
                let property_store = get_property_store();
                let all_properties = property_store.get_all_properties().await;

                if let Some(value) = all_properties.get(&property_key) {
                    let response = PropertyBroadcastMessage::PropertyUpdate {
                        property_key: property_key.clone(),
                        value: value.clone(),
                        validated: true,
                        error: None,
                    };

                    if let Err(e) = self.broadcast_sender.send(response) {
                        warn!("Failed to send property value response: {}", e);
                    }
                } else {
                    let response = PropertyBroadcastMessage::Error {
                        message: format!("Property not found: {}", property_key),
                        property_key: Some(property_key),
                    };

                    if let Err(e) = self.broadcast_sender.send(response) {
                        warn!("Failed to send property not found error: {}", e);
                    }
                }
            }

            PropertyControlMessage::GetAllProperties => {
                let all_properties = get_property_store().get_all_properties().await;
                let response = PropertyBroadcastMessage::PropertyList {
                    properties: all_properties,
                };

                if let Err(e) = self.broadcast_sender.send(response) {
                    warn!("Failed to send all properties response: {}", e);
                }
            }

            PropertyControlMessage::EmergencyStop { reason, applied_by } => {
                error!(
                    "🚨 EMERGENCY STOP requested by {} via WebSocket: {}",
                    applied_by, reason
                );

                // Set emergency stop flags
                let property_store = get_property_store();
                let _ = property_store.set_atomic_bool(
                    "enable_live_execution",
                    false,
                    PropertySource::HumanUI,
                );
                let _ = property_store.set_atomic_bool("dry_run", true, PropertySource::HumanUI);
                let _ = property_store.set_atomic_bool(
                    "simulation_mode",
                    true,
                    PropertySource::HumanUI,
                );

                // Broadcast system health alert
                let health_alert = PropertyBroadcastMessage::SystemHealth {
                    status: "EMERGENCY_STOP".to_string(),
                    message: format!("Emergency stop triggered by {}: {}", applied_by, reason),
                    timestamp: chrono::Utc::now().timestamp() as u64,
                };

                if let Err(e) = self.broadcast_sender.send(health_alert) {
                    warn!("Failed to broadcast emergency stop alert: {}", e);
                }

                metrics::counter!("websocket_emergency_stops_total").increment(1);
            }
        }

        Ok(())
    }

    /// Get current WebSocket connection statistics
    pub async fn get_connection_stats(&self) -> (usize, Vec<String>) {
        let clients = self.clients.read().await;
        let count = clients.len();
        let client_ids: Vec<String> = clients.keys().cloned().collect();
        (count, client_ids)
    }
}

/// WebSocket upgrade handler for Axum
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(broadcaster): State<Arc<WebSocketBroadcaster>>,
) -> Response {
    let client_id = Uuid::new_v4().to_string();

    ws.on_upgrade(move |socket| {
        // Move the Arc into the async task to satisfy lifetime requirements
        let broadcaster = Arc::clone(&broadcaster);
        let client_id = client_id;
        async move {
            broadcaster
                .handle_websocket_connection(socket, client_id)
                .await;
        }
    })
}

/// Create WebSocket broadcaster and start background tasks
pub async fn create_websocket_broadcaster() -> Arc<WebSocketBroadcaster> {
    let mut broadcaster = WebSocketBroadcaster::new();
    broadcaster.start_property_broadcasting().await;

    info!("🔄 WebSocket broadcaster initialized and started");
    Arc::new(broadcaster)
}
