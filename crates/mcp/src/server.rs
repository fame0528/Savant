use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
    Router,
};
use savant_core::error::SavantError;
use savant_skills::parser::SkillRegistry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

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
/// MCP server instance exposing local tools.
pub struct McpServer {
    registry: Arc<RwLock<SkillRegistry>>,
}

impl McpServer {
    /// Starts the server instance.
    pub fn new(registry: Arc<RwLock<SkillRegistry>>) -> Self {
        Self { registry }
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
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&text) {
                let response = match req.method.as_str() {
                    "initialize" => JsonRpcResponse {
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
                    },
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
                    _ => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(
                            serde_json::json!({ "code": -32601, "message": "Method not found" }),
                        ),
                    },
                };

                if let Ok(resp_text) = serde_json::to_string(&response) {
                    let _ = socket.send(Message::Text(resp_text)).await;
                }
            }
        }
    }
}
