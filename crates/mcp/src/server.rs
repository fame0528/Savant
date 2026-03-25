#![allow(clippy::disallowed_methods)] // serde_json::json! macro false positives

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
    Router,
};
use savant_core::error::SavantError;
use savant_skills::parser::SkillRegistry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

/// Rate limiter state per connection
struct ConnectionState {
    request_count: u32,
    last_reset: std::time::Instant,
    authenticated: bool,
}

impl ConnectionState {
    fn new() -> Self {
        Self {
            request_count: 0,
            last_reset: std::time::Instant::now(),
            authenticated: false,
        }
    }

    fn check_rate_limit(&mut self) -> bool {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_reset).as_secs() >= 60 {
            self.request_count = 0;
            self.last_reset = now;
        }
        self.request_count += 1;
        self.request_count <= 100 // 100 requests per minute
    }
}

/// MCP server instance exposing local tools.
pub struct McpServer {
    registry: Arc<RwLock<SkillRegistry>>,
    auth_tokens: HashMap<String, String>, // token_hash -> description
}

impl McpServer {
    /// Starts the server instance.
    pub fn new(registry: Arc<RwLock<SkillRegistry>>) -> Self {
        Self {
            registry,
            auth_tokens: HashMap::new(),
        }
    }

    /// Creates a new MCP server with authentication tokens.
    pub fn with_auth(
        registry: Arc<RwLock<SkillRegistry>>,
        tokens: HashMap<String, String>,
    ) -> Self {
        Self {
            registry,
            auth_tokens: tokens,
        }
    }

    pub async fn start(self: Arc<Self>, addr: &str) -> Result<(), SavantError> {
        let app = Router::new().route(
            "/mcp",
            get(|ws: WebSocketUpgrade| async move {
                ws.on_upgrade(move |socket| handle_socket(socket, self))
            }),
        );

        let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            SavantError::Unknown(format!("Failed to bind MCP server to {}: {}", addr, e))
        })?;

        info!("MCP Server listening on {}", addr);
        axum::serve(listener, app)
            .await
            .map_err(|e| SavantError::Unknown(format!("MCP Server runtime error: {}", e)))?;

        Ok(())
    }
}

async fn handle_socket(mut socket: WebSocket, server: Arc<McpServer>) {
    let mut state = ConnectionState::new();

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            // Rate limiting
            if !state.check_rate_limit() {
                let err_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: serde_json::json!(null),
                    result: None,
                    error: Some(serde_json::json!({
                        "code": -32000,
                        "message": "Rate limit exceeded (100 req/min)"
                    })),
                };
                if let Ok(resp_text) = serde_json::to_string(&err_response) {
                    if let Err(e) = socket.send(Message::Text(resp_text)).await {
                        warn!(
                            "[mcp::server] Failed to send rate limit error response: {}",
                            e
                        );
                    }
                }
                continue;
            }

            let req: JsonRpcRequest = match serde_json::from_str(&text) {
                Ok(r) => r,
                Err(_) => {
                    let err_response = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: serde_json::json!(null),
                        result: None,
                        error: Some(serde_json::json!({
                            "code": -32700,
                            "message": "Parse error"
                        })),
                    };
                    if let Ok(resp_text) = serde_json::to_string(&err_response) {
                        if let Err(e) = socket.send(Message::Text(resp_text)).await {
                            warn!("[mcp::server] Failed to send parse error response: {}", e);
                        }
                    }
                    continue;
                }
            };

            // Authentication check: require auth before any method except initialize
            if req.method != "initialize" && !state.authenticated {
                if !server.auth_tokens.is_empty() {
                    let err_response = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(serde_json::json!({
                            "code": -32002,
                            "message": "Authentication required. Call 'initialize' with auth_token."
                        })),
                    };
                    if let Ok(resp_text) = serde_json::to_string(&err_response) {
                        if let Err(e) = socket.send(Message::Text(resp_text)).await {
                            warn!("[mcp::server] Failed to send auth required response: {}", e);
                        }
                    }
                    continue;
                }
                // No auth tokens configured = allow all (dev mode)
                state.authenticated = true;
            }

            let response = match req.method.as_str() {
                "initialize" => {
                    // Check auth token if server has tokens configured
                    let auth_ok = if !server.auth_tokens.is_empty() {
                        let provided_token = req
                            .params
                            .as_ref()
                            .and_then(|p| p.get("auth_token"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("");

                        if provided_token.is_empty() {
                            // No token provided but auth is configured
                            if let Err(e) = socket.send(Message::Text(
                                serde_json::to_string(&JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: req.id.clone(),
                                    result: None,
                                    error: Some(serde_json::json!({
                                        "code": -32002,
                                        "message": "Authentication required: provide auth_token in initialize params"
                                    })),
                                }).unwrap_or_default()
                            )).await {
                                warn!("[mcp::server] Failed to send auth required response: {}", e);
                            }
                            continue;
                        }

                        let token_hash = {
                            use std::hash::Hasher;
                            let mut hasher = std::collections::hash_map::DefaultHasher::new();
                            hasher.write(provided_token.as_bytes());
                            format!("{:x}", hasher.finish())
                        };

                        if server.auth_tokens.contains_key(&token_hash) {
                            info!("MCP client authenticated");
                            true
                        } else {
                            warn!("MCP authentication failed: invalid token");
                            if let Err(e) = socket
                                .send(Message::Text(
                                    serde_json::to_string(&JsonRpcResponse {
                                        jsonrpc: "2.0".to_string(),
                                        id: req.id.clone(),
                                        result: None,
                                        error: Some(serde_json::json!({
                                            "code": -32001,
                                            "message": "Authentication failed: invalid token"
                                        })),
                                    })
                                    .unwrap_or_default(),
                                ))
                                .await
                            {
                                warn!("[mcp::server] Failed to send auth failed response: {}", e);
                            }
                            continue;
                        }
                    } else {
                        true // No auth configured
                    };

                    state.authenticated = auth_ok;

                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(serde_json::json!({
                            "protocolVersion": "2024-11-05",
                            "capabilities": {
                                "tools": { "listChanged": false }
                            },
                            "serverInfo": { "name": "Savant MCP Server", "version": "1.0.0" }
                        })),
                        error: None,
                    }
                }
                "tools/list" => {
                    let registry = server.registry.read().await;
                    let tools: Vec<Value> = registry
                        .manifests
                        .values()
                        .map(|m| {
                            serde_json::json!({
                                "name": m.name,
                                "description": m.description,
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {}
                                }
                            })
                        })
                        .collect();

                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(serde_json::json!({ "tools": tools })),
                        error: None,
                    }
                }
                "tools/call" => {
                    let name = req
                        .params
                        .as_ref()
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("");
                    let args = req
                        .params
                        .as_ref()
                        .and_then(|p| p.get("arguments"))
                        .cloned()
                        .unwrap_or(serde_json::json!({}));

                    // Validate tool name
                    if name.is_empty() || name.len() > 128 {
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: None,
                            error: Some(serde_json::json!({
                                "code": -32602,
                                "message": "Invalid tool name"
                            })),
                        }
                    } else {
                        let registry = server.registry.read().await;
                        if let Some(tool) = registry.tools.get(name) {
                            match tool.execute(args).await {
                                Ok(content) => JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: req.id,
                                    result: Some(serde_json::json!({
                                        "content": [{ "type": "text", "text": content }]
                                    })),
                                    error: None,
                                },
                                Err(e) => JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: req.id,
                                    result: None,
                                    error: Some(serde_json::json!({
                                        "code": -32000,
                                        "message": format!("Tool execution failed: {}", e)
                                    })),
                                },
                            }
                        } else {
                            JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: req.id,
                                result: None,
                                error: Some(
                                    serde_json::json!({ "code": -32601, "message": "Method not found" }),
                                ),
                            }
                        }
                    }
                }
                _ => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: None,
                    error: Some(
                        serde_json::json!({ "code": -32601, "message": "Method not found" }),
                    ),
                },
            };

            match serde_json::to_string(&response) {
                Ok(resp_text) => {
                    if let Err(e) = socket.send(Message::Text(resp_text)).await {
                        warn!("MCP send error: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    warn!("MCP response serialization error: {}", e);
                }
            }
        }
    }
}
