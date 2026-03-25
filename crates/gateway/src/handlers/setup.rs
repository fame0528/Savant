//! Setup wizard handlers for first-launch dependency checks.

use crate::server::GatewayState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::sync::Arc;

/// GET /api/setup/check — Check Ollama + model availability
pub async fn setup_check_handler(State(_state): State<Arc<GatewayState>>) -> impl IntoResponse {
    let mut checks = serde_json::json!({
        "ollama_running": false,
        "ollama_installed": false,
        "model_available": false,
        "model_name": "qwen3-embedding:4b",
        "issues": [],
        "instructions": [],
    });

    let mut issues: Vec<String> = Vec::new();
    let mut instructions: Vec<String> = Vec::new();

    // Check 1: Is Ollama running?
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());

    match savant_core::net::secure_client()
        .get(format!("{}/api/tags", ollama_url))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            checks["ollama_running"] = serde_json::Value::Bool(true);
            checks["ollama_installed"] = serde_json::Value::Bool(true);

            // Check 2: Is the embedding model available?
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                let models = body["models"].as_array().cloned().unwrap_or_default();
                let has_model = models
                    .iter()
                    .any(|m| m["name"].as_str().unwrap_or("").contains("qwen3-embedding"));

                checks["model_available"] = serde_json::Value::Bool(has_model);

                if !has_model {
                    issues.push("qwen3-embedding:4b model not found in Ollama".to_string());
                    instructions.push("Run: ollama pull qwen3-embedding:4b".to_string());
                }
            }
        }
        Ok(resp) => {
            issues.push(format!("Ollama returned status {}", resp.status()));
            instructions
                .push("Ollama is running but returned an error. Try restarting it.".to_string());
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("Connection refused") || err_str.contains("connect error") {
                issues.push("Ollama is not running".to_string());
                instructions.push(
                    "Start Ollama: ollama serve (or install from https://ollama.com)".to_string(),
                );
            } else {
                issues.push(format!("Cannot connect to Ollama: {}", err_str));
                instructions
                    .push("Check if Ollama is installed and running on port 11434".to_string());
            }
        }
    }

    checks["issues"] =
        serde_json::Value::Array(issues.into_iter().map(serde_json::Value::String).collect());
    checks["instructions"] = serde_json::Value::Array(
        instructions
            .into_iter()
            .map(serde_json::Value::String)
            .collect(),
    );

    Json(checks).into_response()
}

/// POST /api/setup/install-model — Pull the embedding model via Ollama
pub async fn setup_install_model_handler(
    State(_state): State<Arc<GatewayState>>,
) -> impl IntoResponse {
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());

    match savant_core::net::secure_client()
        .post(format!("{}/api/pull", ollama_url))
        .json(&serde_json::json!({
            "name": "qwen3-embedding:4b",
            "stream": false
        }))
        .timeout(std::time::Duration::from_secs(600))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => Json(serde_json::json!({
            "status": "success",
            "message": "qwen3-embedding:4b installed successfully"
        }))
        .into_response(),
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "status": "error",
                    "message": format!("Ollama pull failed ({}): {}", status, body)
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "error",
                "message": format!("Cannot connect to Ollama: {}", e)
            })),
        )
            .into_response(),
    }
}
