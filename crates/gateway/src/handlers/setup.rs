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

/// GET /api/setup/check — Check Ollama/LM Studio + model availability
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
        "providers": [],
    });

    let mut issues: Vec<String> = Vec::new();
    let mut instructions: Vec<String> = Vec::new();
    let mut providers: Vec<serde_json::Value> = Vec::new();

    // Check Ollama
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let ollama_result = check_provider(&ollama_url, "Ollama", &configured_model).await;
    if ollama_result.running {
        checks["ollama_running"] = serde_json::Value::Bool(true);
        checks["ollama_installed"] = serde_json::Value::Bool(true);
        if ollama_result.model_available {
            checks["model_available"] = serde_json::Value::Bool(true);
        }
    }
    providers.push(serde_json::json!({
        "name": "Ollama",
        "url": ollama_url,
        "running": ollama_result.running,
        "model_available": ollama_result.model_available,
        "error": ollama_result.error,
    }));

    // Check LM Studio
    let lmstudio_url =
        std::env::var("LMSTUDIO_URL").unwrap_or_else(|_| "http://localhost:1234".to_string());
    let lmstudio_result = check_provider(&lmstudio_url, "LM Studio", &configured_model).await;
    if lmstudio_result.running {
        checks["ollama_running"] = serde_json::Value::Bool(true);
        checks["ollama_installed"] = serde_json::Value::Bool(true);
        if lmstudio_result.model_available {
            checks["model_available"] = serde_json::Value::Bool(true);
        }
    }
    providers.push(serde_json::json!({
        "name": "LM Studio",
        "url": lmstudio_url,
        "running": lmstudio_result.running,
        "model_available": lmstudio_result.model_available,
        "error": lmstudio_result.error,
    }));

    // If neither is running, populate issues
    if !ollama_result.running && !lmstudio_result.running {
        issues.push("No local AI provider detected".to_string());
        instructions.push("Install Ollama (https://ollama.com/download) or LM Studio (https://lmstudio.ai)".to_string());
        instructions.push("Start the provider and return here.".to_string());
    } else if !checks["model_available"].as_bool().unwrap_or(false) {
        issues.push(format!("Model '{}' not found", configured_model));
        instructions.push("Select a model variant during setup to auto-install.".to_string());
    }

    checks["issues"] =
        serde_json::Value::Array(issues.into_iter().map(serde_json::Value::String).collect());
    checks["instructions"] = serde_json::Value::Array(
        instructions
            .into_iter()
            .map(serde_json::Value::String)
            .collect(),
    );
    checks["providers"] = serde_json::Value::Array(providers);

    Json(checks).into_response()
}

struct ProviderCheck {
    running: bool,
    model_available: bool,
    error: Option<String>,
}

async fn check_provider(url: &str, name: &str, configured_model: &str) -> ProviderCheck {
    // Use a longer timeout for local providers — first response can be slow
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .connect_timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ProviderCheck {
                running: false,
                model_available: false,
                error: Some(format!("{} client build failed: {}", name, e)),
            };
        }
    };

    // Try /api/tags (Ollama) first, then /v1/models (LM Studio / OpenAI-compatible)
    let tags_url = format!("{}/api/tags", url);
    let models_url = format!("{}/v1/models", url);

    // Try Ollama-style endpoint
    match client.get(&tags_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                let models = body["models"].as_array().cloned().unwrap_or_default();
                let has_model = models.iter().any(|m| {
                    let name = m["name"].as_str().unwrap_or("");
                    name == configured_model || name.starts_with(configured_model)
                });
                return ProviderCheck {
                    running: true,
                    model_available: has_model,
                    error: None,
                };
            }
            return ProviderCheck {
                running: true,
                model_available: false,
                error: None,
            };
        }
        Ok(resp) => {
            // Try LM Studio / OpenAI-compatible endpoint
            match client.get(&models_url).send().await {
                Ok(resp2) if resp2.status().is_success() => {
                    if let Ok(body) = resp2.json::<serde_json::Value>().await {
                        let data = body["data"].as_array().cloned().unwrap_or_default();
                        let has_model = data.iter().any(|m| {
                            let id = m["id"].as_str().unwrap_or("");
                            id == configured_model || id.starts_with(configured_model)
                        });
                        return ProviderCheck {
                            running: true,
                            model_available: has_model,
                            error: None,
                        };
                    }
                    return ProviderCheck {
                        running: true,
                        model_available: false,
                        error: None,
                    };
                }
                Ok(resp2) => {
                    return ProviderCheck {
                        running: false,
                        model_available: false,
                        error: Some(format!("{} returned status {}", name, resp2.status())),
                    };
                }
                Err(e2) => {
                    return ProviderCheck {
                        running: false,
                        model_available: false,
                        error: Some(format!("{} not reachable: {}", name, e2)),
                    };
                }
            }
        }
        Err(e) => {
            // Try LM Studio / OpenAI-compatible endpoint as fallback
            match client.get(&models_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        let data = body["data"].as_array().cloned().unwrap_or_default();
                        let has_model = data.iter().any(|m| {
                            let id = m["id"].as_str().unwrap_or("");
                            id == configured_model || id.starts_with(configured_model)
                        });
                        return ProviderCheck {
                            running: true,
                            model_available: has_model,
                            error: None,
                        };
                    }
                    return ProviderCheck {
                        running: true,
                        model_available: false,
                        error: None,
                    };
                }
                Ok(resp) => {
                    return ProviderCheck {
                        running: false,
                        model_available: false,
                        error: Some(format!("{} returned status {:?} (Ollama) and {:?} (OpenAI)", name, e.status(), resp.status())),
                    };
                }
                Err(e2) => {
                    let err_str = format!("{} / {}", e, e2);
                    if err_str.contains("Connection refused") || err_str.contains("connect error") || err_str.contains("timed out") {
                        return ProviderCheck {
                            running: false,
                            model_available: false,
                            error: Some(format!("{} is not running", name)),
                        };
                    }
                    return ProviderCheck {
                        running: false,
                        model_available: false,
                        error: Some(format!("Cannot connect to {}: {}", name, err_str)),
                    };
                }
            }
        }
    }
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
