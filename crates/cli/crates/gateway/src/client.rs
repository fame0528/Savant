//! Gateway client — WebSocket connection management with exponential backoff,
//! ping/pong keepalive, and split read/write tasks.

use anyhow::{Context, Result};
use backoff::{backoff::Backoff, ExponentialBackoff};
use futures::{SinkExt, StreamExt};
use savant_core::types::{ControlFrame, EventFrame, RequestFrame, RequestPayload, SessionId};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{protocol::frame::coding::CloseCode, Message},
};
use tracing::{debug, error, info, warn};

use crate::auth::Ed25519Auth;
use crate::events::EventRouter;
use crate::handlers::ControlFrameHandler;

/// Gateway connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Authenticating,
    Reconnecting,
}

/// Gateway client configuration
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// Gateway WebSocket URL (e.g., "ws://localhost:3000/ws")
    pub url: String,
    /// Optional API key for fallback authentication
    pub api_key: Option<String>,
    /// Ping interval in seconds (default: 15)
    pub ping_interval_secs: u64,
    /// Maximum reconnection attempts (None = infinite)
    pub max_reconnect_attempts: Option<u32>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            url: "ws://localhost:3000/ws".to_string(),
            api_key: None,
            ping_interval_secs: 15,
            max_reconnect_attempts: None,
        }
    }
}

/// Internal message types for the gateway client
#[derive(Debug)]
enum InternalMessage {
    /// Send a RequestFrame to the gateway
    SendRequest(Box<RequestFrame>),
    /// Close the connection gracefully
    Close,
}

/// Gateway client — manages WebSocket connection to Savant Gateway
pub struct GatewayClient {
    config: GatewayConfig,
    state: Arc<RwLock<ConnectionState>>,
    connected: Arc<AtomicBool>,
    tx: Arc<RwLock<Option<mpsc::Sender<InternalMessage>>>>,
    event_router: EventRouter,
    handler: ControlFrameHandler,
    auth: Option<Ed25519Auth>,
    /// Background task handles for cleanup on disconnect.
    task_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl GatewayClient {
    /// Create a new gateway client
    pub fn new(config: GatewayConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            connected: Arc::new(AtomicBool::new(false)),
            tx: Arc::new(RwLock::new(None)),
            event_router: EventRouter::new(),
            handler: ControlFrameHandler::new(),
            auth: None,
            task_handles: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Set Ed25519 authentication credentials
    pub fn with_auth(mut self, auth: Ed25519Auth) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Get the current connection state
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Connect to the gateway with exponential backoff
    pub async fn connect(&self) -> Result<()> {
        let mut backoff = ExponentialBackoff {
            initial_interval: Duration::from_secs(1),
            max_interval: Duration::from_secs(16),
            multiplier: 2.0,
            max_elapsed_time: None,
            ..Default::default()
        };

        let mut attempts = 0u32;

        loop {
            *self.state.write().await = if attempts == 0 {
                ConnectionState::Connecting
            } else {
                ConnectionState::Reconnecting
            };

            match self.try_connect().await {
                Ok(_) => {
                    info!("Connected to Savant Gateway at {}", self.config.url);
                    *self.state.write().await = ConnectionState::Connected;
                    self.connected.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                Err(e) => {
                    attempts += 1;
                    if let Some(max) = self.config.max_reconnect_attempts {
                        if attempts >= max {
                            error!("Failed to connect after {} attempts: {}", attempts, e);
                            *self.state.write().await = ConnectionState::Disconnected;
                            return Err(e);
                        }
                    }

                    if let Some(delay) = backoff.next_backoff() {
                        warn!(
                            "Connection attempt {} failed: {}. Retrying in {:?}...",
                            attempts, e, delay
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        return Err(e.context("Backoff exhausted"));
                    }
                }
            }
        }
    }

    /// Attempt a single connection
    async fn try_connect(&self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.config.url)
            .await
            .with_context(|| format!("Failed to connect to {}", self.config.url))?;

        let (mut write, mut read) = ws_stream.split();

        // RC-15: Bounded channel for backpressure
        let (internal_tx, mut internal_rx) = mpsc::channel::<InternalMessage>(500);
        *self.tx.write().await = Some(internal_tx);

        // Authenticate
        if let Some(auth) = &self.auth {
            *self.state.write().await = ConnectionState::Authenticating;
            let auth_frame = auth.create_auth_frame()?;
            let auth_json = serde_json::to_string(&auth_frame)?;
            write
                .send(Message::Text(auth_json.into()))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send auth frame: {}", e))?;
        } else if let Some(api_key) = &self.config.api_key {
            *self.state.write().await = ConnectionState::Authenticating;
            let auth_frame = RequestFrame {
                request_id: uuid::Uuid::new_v4().to_string(),
                session_id: SessionId("cli-auth".to_string()),
                payload: RequestPayload::Auth(api_key.clone()),
                signature: None,
                timestamp: Some(chrono::Utc::now().timestamp_millis()),
            };
            let auth_json = serde_json::to_string(&auth_frame)?;
            write
                .send(Message::Text(auth_json.into()))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send auth frame: {}", e))?;
        }

        let connected = self.connected.clone();
        let state = self.state.clone();
        let event_router = self.event_router.clone();
        let handler = self.handler.clone();

        // Spawn read task
        let read_handle = tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = Self::handle_message(&text, &event_router, &handler).await {
                            warn!("Error handling message: {}", e);
                        }
                    }
                    Ok(Message::Ping(_data)) => {
                        debug!("Received ping");
                        // Pong is handled automatically by tungstenite
                    }
                    Ok(Message::Pong(_)) => {
                        debug!("Received pong");
                    }
                    Ok(Message::Close(frame)) => {
                        info!("Gateway closed: {:?}", frame.as_ref().map(|f| &f.reason));
                        connected.store(false, Ordering::SeqCst);
                        *state.write().await = ConnectionState::Disconnected;
                        break;
                    }
                    Ok(Message::Binary(data)) => {
                        debug!("Received binary message ({} bytes)", data.len());
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        connected.store(false, Ordering::SeqCst);
                        *state.write().await = ConnectionState::Disconnected;
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Spawn ping task
        let ping_interval = self.config.ping_interval_secs;
        let ping_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(ping_interval));
            loop {
                interval.tick().await;
                debug!("Sending ping");
                // tungstenite handles ping/pong automatically
            }
        });

        // Spawn write task
        let write_handle = tokio::spawn(async move {
            let mut write = write;
            while let Some(msg) = internal_rx.recv().await {
                match msg {
                    InternalMessage::SendRequest(frame) => match serde_json::to_string(&*frame) {
                        Ok(json) => {
                            if let Err(e) = write.send(Message::Text(json.into())).await {
                                error!("Failed to send message: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Failed to serialize frame: {}", e);
                        }
                    },
                    InternalMessage::Close => {
                        if let Err(e) = write
                            .send(Message::Close(Some(
                                tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                    code: CloseCode::Normal,
                                    reason: "Client closing".into(),
                                },
                            )))
                            .await
                        {
                            error!("Failed to send close frame: {}", e);
                        }
                        break;
                    }
                }
            }
        });

        // Store handles for cleanup on disconnect
        {
            let mut handles = self.task_handles.lock().await;
            handles.push(read_handle);
            handles.push(ping_handle);
            handles.push(write_handle);
        }

        Ok(())
    }

    /// Handle an incoming message
    async fn handle_message(
        text: &str,
        event_router: &EventRouter,
        handler: &ControlFrameHandler,
    ) -> Result<()> {
        // Check if it's an event (prefixed with "EVENT:")
        if let Some(event_json) = text.strip_prefix("EVENT:") {
            let event: EventFrame = serde_json::from_str(event_json)
                .with_context(|| format!("Failed to parse event: {}", event_json))?;
            event_router.route(event).await;
            return Ok(());
        }

        // Otherwise, try to parse as a response frame
        if let Ok(frame) = serde_json::from_str::<RequestFrame>(text) {
            handler.handle_response(frame).await?;
            return Ok(());
        }

        // Try parsing as a raw JSON response
        debug!("Received raw JSON: {}", text);
        Ok(())
    }

    /// Send a request frame to the gateway
    pub async fn send(&self, frame: RequestFrame) -> Result<()> {
        let tx = self.tx.read().await;
        if let Some(tx) = tx.as_ref() {
            // RC-15: Use try_send for bounded channel
            tx.try_send(InternalMessage::SendRequest(Box::new(frame)))
                .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Not connected to gateway"))
        }
    }

    /// Send a control frame
    pub async fn send_control(&self, control: ControlFrame) -> Result<()> {
        let frame = RequestFrame {
            request_id: uuid::Uuid::new_v4().to_string(),
            session_id: SessionId("cli-session".to_string()),
            payload: RequestPayload::ControlFrame(control),
            signature: None,
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
        };
        self.send(frame).await
    }

    /// Subscribe to events of a specific type
    pub async fn subscribe<F>(&self, event_type: &str, handler: F)
    where
        F: Fn(EventFrame) + Send + Sync + 'static,
    {
        self.event_router
            .subscribe(event_type.to_string(), handler)
            .await;
    }

    /// Disconnect from the gateway
    pub async fn disconnect(&self) {
        let tx = self.tx.read().await;
        if let Some(tx) = tx.as_ref() {
            if let Err(e) = tx.try_send(InternalMessage::Close) {
                error!("Failed to send disconnect signal: {}", e);
            }
        }
        self.connected.store(false, Ordering::SeqCst);
        *self.state.write().await = ConnectionState::Disconnected;

        // Abort background tasks
        let mut handles = self.task_handles.lock().await;
        for handle in handles.drain(..) {
            handle.abort();
        }
    }
}
