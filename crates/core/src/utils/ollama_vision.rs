use crate::error::SavantError;
use crate::traits::VisionProvider;
use async_trait::async_trait;
use tracing::{info, warn};

const DEFAULT_MODEL: &str = "qwen3-vl";
const DEFAULT_URL: &str = "http://localhost:11434";

/// Vision service that uses Ollama for image understanding.
pub struct OllamaVisionService {
    client: reqwest::Client,
    url: String,
    model: String,
}

impl OllamaVisionService {
    pub fn new() -> Self {
        let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
        let model =
            std::env::var("OLLAMA_VISION_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
        info!(
            "Initializing OllamaVisionService (model={}, url={})",
            model, url
        );
        Self {
            client: crate::net::secure_client(),
            url,
            model,
        }
    }

    pub fn with_config(url: &str, model: &str) -> Self {
        info!(
            "Initializing OllamaVisionService (model={}, url={})",
            model, url
        );
        Self {
            client: crate::net::secure_client(),
            url: url.to_string(),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl VisionProvider for OllamaVisionService {
    async fn describe_image(
        &self,
        image_base64: &str,
        prompt: &str,
    ) -> Result<String, SavantError> {
        let resp: serde_json::Value = self
            .client
            .post(format!("{}/api/generate", self.url))
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": prompt,
                "images": [image_base64],
                "stream": false
            }))
            .send()
            .await
            .map_err(|e| SavantError::Unknown(format!("Ollama vision request failed: {}", e)))?
            .json()
            .await
            .map_err(|e| {
                SavantError::Unknown(format!("Ollama vision response parse failed: {}", e))
            })?;

        let response = resp["response"].as_str().ok_or_else(|| {
            SavantError::Unknown("No response in Ollama vision result".to_string())
        })?;

        Ok(response.to_string())
    }

    async fn is_available(&self) -> bool {
        match self
            .client
            .get(format!("{}/api/tags", self.url))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    let models = body["models"].as_array().cloned().unwrap_or_default();
                    return models
                        .iter()
                        .any(|m| m["name"].as_str().unwrap_or("").contains("qwen3-vl"));
                }
                false
            }
            _ => false,
        }
    }
}

/// Try to create a vision service. Returns None if Ollama is unavailable or model not found.
pub async fn create_vision_service() -> Option<Box<dyn VisionProvider>> {
    let svc = OllamaVisionService::new();
    if svc.is_available().await {
        info!("Ollama qwen3-vl model found, using Ollama vision");
        Some(Box::new(svc))
    } else {
        warn!("Ollama vision unavailable (model qwen3-vl not found or Ollama not running)");
        None
    }
}
