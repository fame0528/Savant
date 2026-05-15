//! Setup wizard handlers for first-launch dependency checks and config.

use crate::server::GatewayState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct InstallModelRequest {
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigSetRequest {
    pub section: String,
    pub key: String,
    pub value: serde_json::Value,
}

/// POST /api/config/set — Update a config value and save to disk
pub async fn config_set_handler(
    State(_state): State<Arc<GatewayState>>,
    Json(body): Json<ConfigSetRequest>,
) -> impl IntoResponse {
    let config_path = savant_core::config::Config::primary_config_path();

    let mut config = match savant_core::config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to load config: {}", e)
                })),
            )
                .into_response();
        }
    };

    let result = match body.section.as_str() {
        "browser" => match body.key.as_str() {
            "vision_model" => {
                config.browser.vision_model = body.value.as_str().unwrap_or("gemma4").to_string();
                Ok(())
            }
            "embedding_model" => {
                config.browser.embedding_model = body.value.as_str().unwrap_or("gemma4").to_string();
                Ok(())
            }
            "vision_model_provider" => {
                config.browser.vision_model_provider = body.value.as_str().unwrap_or("ollama").to_string();
                Ok(())
            }
            "enabled" => {
                config.browser.enabled = body.value.as_bool().unwrap_or(true);
                Ok(())
            }
            _ => Err(format!("Unknown browser key: {}", body.key)),
        },
        "obsidian" => match body.key.as_str() {
            "vault_path" => {
                config.obsidian.vault_path = body.value.as_str().map(|s| s.to_string());
                Ok(())
            }
            "enabled" => {
                config.obsidian.enabled = body.value.as_bool().unwrap_or(true);
                Ok(())
            }
            "sync_interval_secs" => {
                config.obsidian.sync_interval_secs = body.value.as_u64().unwrap_or(300);
                Ok(())
            }
            _ => Err(format!("Unknown obsidian key: {}", body.key)),
        },
        "ai" => match body.key.as_str() {
            "model" => {
                config.ai.model = body.value.as_str().unwrap_or("").to_string();
                Ok(())
            }
            "provider" => {
                config.ai.provider = body.value.as_str().unwrap_or("ollama").to_string();
                Ok(())
            }
            _ => Err(format!("Unknown ai key: {}", body.key)),
        },
        _ => Err(format!("Unknown config section: {}", body.section)),
    };

    match result {
        Ok(()) => {
            if let Err(e) = config.save(&config_path) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "status": "error",
                        "message": format!("Failed to save config: {}", e)
                    })),
                )
                    .into_response();
            }
            Json(serde_json::json!({
                "status": "success",
                "section": body.section,
                "key": body.key,
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "status": "error",
                "message": e
            })),
        )
            .into_response(),
    }
}

/// GET /api/setup/check — Check Ollama + model availability
pub async fn setup_check_handler(State(_state): State<Arc<GatewayState>>) -> impl IntoResponse {
    let configured_model = savant_core::config::Config::load()
        .map(|c| c.browser.embedding_model.clone())
        .unwrap_or_else(|_| "gemma4".to_string());

    let mut checks = serde_json::json!({
        "ollama_running": false,
        "ollama_installed": false,
        "model_available": false,
        "model_name": configured_model,
        "issues": [],
        "instructions": [],
    });

    let mut issues: Vec<String> = Vec::new();
    let mut instructions: Vec<String> = Vec::new();

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

            if let Ok(body) = resp.json::<serde_json::Value>().await {
                let models = body["models"].as_array().cloned().unwrap_or_default();
                let has_model = models
                    .iter()
                    .any(|m| {
                        let name = m["name"].as_str().unwrap_or("");
                        name == configured_model || name.starts_with(&configured_model)
                    });

                checks["model_available"] = serde_json::Value::Bool(has_model);

                if !has_model {
                    issues.push("Gemma 4 model not found in Ollama".to_string());
                    instructions.push("Select a Gemma 4 variant during setup to auto-install.".to_string());
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
                    "Install Ollama from https://ollama.com/download and start it.".to_string(),
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

/// POST /api/setup/install-model — Pull a model via Ollama
/// Body: { "model": "gemma4:e4b" }
pub async fn setup_install_model_handler(
    State(_state): State<Arc<GatewayState>>,
    Json(body): Json<InstallModelRequest>,
) -> impl IntoResponse {
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());

    match savant_core::net::secure_client()
        .post(format!("{}/api/pull", ollama_url))
        .json(&serde_json::json!({
            "name": body.model,
            "stream": false
        }))
        .timeout(std::time::Duration::from_secs(600))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => Json(serde_json::json!({
            "status": "success",
            "message": format!("{} installed successfully", body.model)
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
