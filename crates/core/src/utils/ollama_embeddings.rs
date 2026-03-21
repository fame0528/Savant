use crate::error::SavantError;
use crate::traits::EmbeddingProvider;
use async_trait::async_trait;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use tracing::{info, warn};

const CACHE_CAPACITY: NonZeroUsize = match NonZeroUsize::new(1000) {
    Some(v) => v,
    None => unreachable!(),
};

const DEFAULT_MODEL: &str = "qwen3-embedding:4b";
const DEFAULT_URL: &str = "http://localhost:11434";

/// Embedding service that uses Ollama for high-quality embeddings.
/// Falls back to local fastembed if Ollama is unavailable.
pub struct OllamaEmbeddingService {
    client: reqwest::Client,
    url: String,
    model: String,
    cache: Mutex<LruCache<String, Vec<f32>>>,
}

impl OllamaEmbeddingService {
    pub fn new() -> Result<Self, SavantError> {
        let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
        let model =
            std::env::var("OLLAMA_EMBED_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
        info!(
            "Initializing OllamaEmbeddingService (model={}, url={})",
            model, url
        );
        Ok(Self {
            client: reqwest::Client::new(),
            url,
            model,
            cache: Mutex::new(LruCache::new(CACHE_CAPACITY)),
        })
    }

    pub fn with_config(url: &str, model: &str) -> Self {
        info!(
            "Initializing OllamaEmbeddingService (model={}, url={})",
            model, url
        );
        Self {
            client: reqwest::Client::new(),
            url: url.to_string(),
            model: model.to_string(),
            cache: Mutex::new(LruCache::new(CACHE_CAPACITY)),
        }
    }

    async fn call_ollama(&self, text: &str) -> Result<Vec<f32>, SavantError> {
        let resp: serde_json::Value = self
            .client
            .post(format!("{}/api/embeddings", self.url))
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": text
            }))
            .send()
            .await
            .map_err(|e| SavantError::Unknown(format!("Ollama request failed: {}", e)))?
            .json()
            .await
            .map_err(|e| SavantError::Unknown(format!("Ollama response parse failed: {}", e)))?;

        let embedding = resp["embedding"]
            .as_array()
            .ok_or_else(|| SavantError::Unknown("No embedding in Ollama response".to_string()))?
            .iter()
            .filter_map(|v| v.as_f64())
            .map(|f| f as f32)
            .collect::<Vec<f32>>();

        if embedding.is_empty() || embedding.iter().all(|&v| v == 0.0) {
            return Err(SavantError::Unknown(
                "Ollama returned zero embedding".to_string(),
            ));
        }

        Ok(embedding)
    }

    pub fn dimensions(&self) -> usize {
        // qwen3-embedding:4b outputs 2560 dimensions
        // (varies by model, but we'll detect at runtime if needed)
        2560
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingService {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, SavantError> {
        // Check cache
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| SavantError::Unknown("Cache lock poisoned".to_string()))?;
            if let Some(cached) = cache.get(text) {
                return Ok(cached.clone());
            }
        }

        let embedding = self.call_ollama(text).await?;

        // Cache result
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| SavantError::Unknown("Cache lock poisoned".to_string()))?;
            cache.put(text.to_string(), embedding.clone());
        }

        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SavantError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    fn dimensions(&self) -> usize {
        self.dimensions()
    }
}

/// Tries Ollama first, falls back to fastembed if Ollama is unavailable.
pub async fn create_embedding_service() -> Result<Box<dyn EmbeddingProvider>, SavantError> {
    // Try Ollama first
    let ollama = OllamaEmbeddingService::new()?;
    // Quick health check
    match ollama
        .client
        .get(format!("{}/api/tags", ollama.url))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            // Check if the embedding model is available
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                let models = body["models"].as_array().cloned().unwrap_or_default();
                let has_model = models
                    .iter()
                    .any(|m| m["name"].as_str().unwrap_or("").contains("qwen3-embedding"));
                if has_model {
                    info!("Ollama qwen3-embedding model found, using Ollama embeddings");
                    return Ok(Box::new(ollama));
                } else {
                    warn!("Ollama running but qwen3-embedding model not found. Pull it with: ollama pull qwen3-embedding:4b");
                }
            }
        }
        _ => {
            warn!(
                "Ollama not available at {}, falling back to fastembed",
                ollama.url
            );
        }
    }

    // Fallback to fastembed
    info!("Falling back to fastembed (AllMiniLML6V2)");
    match super::embeddings::EmbeddingService::new() {
        Ok(svc) => Ok(Box::new(svc)),
        Err(e) => Err(SavantError::Unknown(format!(
            "Both Ollama and fastembed failed: {}",
            e
        ))),
    }
}
