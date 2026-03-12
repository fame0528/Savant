pub mod mgmt;
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use savant_core::error::SavantError;
use reqwest::Client;
use serde_json::json;

/// Abstraction for a language model service provider.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Request a chat completion, returning a streamed byte response.
    async fn stream_completion(&self, messages: Vec<serde_json::Value>) -> Result<Box<dyn Stream<Item = Result<bytes::Bytes, SavantError>> + Send + Unpin>, SavantError>;
}

/// OpenAI implementation of LlmProvider.
pub struct OpenAiProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn stream_completion(&self, messages: Vec<serde_json::Value>) -> Result<Box<dyn Stream<Item = Result<bytes::Bytes, SavantError>> + Send + Unpin>, SavantError> {
        let response = self.client.post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("OpenAI request failed: {}", e)))?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(SavantError::AuthError(format!("OpenAI API error: {}", err_text)));
        }

        let stream = response.bytes_stream().map(|res| {
            res.map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))
        });

        Ok(Box::new(stream))
    }
}

/// Anthropic implementation of LlmProvider.
pub struct AnthropicProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn stream_completion(&self, messages: Vec<serde_json::Value>) -> Result<Box<dyn Stream<Item = Result<bytes::Bytes, SavantError>> + Send + Unpin>, SavantError> {
        let response = self.client.post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
                "max_tokens": 4096,
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Anthropic request failed: {}", e)))?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(SavantError::AuthError(format!("Anthropic API error: {}", err_text)));
        }

        let stream = response.bytes_stream().map(|res| {
            res.map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))
        });

        Ok(Box::new(stream))
    }
}

/// Ollama implementation of LlmProvider.
pub struct OllamaProvider {
    pub client: Client,
    pub url: String, // e.g. "http://localhost:11434"
    pub model: String,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn stream_completion(&self, messages: Vec<serde_json::Value>) -> Result<Box<dyn Stream<Item = Result<bytes::Bytes, SavantError>> + Send + Unpin>, SavantError> {
        let response = self.client.post(format!("{}/api/chat", self.url))
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Ollama request failed: {}", e)))?;

        let stream = response.bytes_stream().map(|res| {
            res.map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))
        });

        Ok(Box::new(stream))
    }
}

/// LM Studio implementation (OpenAI-compatible).
pub struct LmStudioProvider {
    pub client: Client,
    pub url: String, // e.g. "http://localhost:1234/v1"
    pub model: String,
}

#[async_trait]
impl LlmProvider for LmStudioProvider {
    async fn stream_completion(&self, messages: Vec<serde_json::Value>) -> Result<Box<dyn Stream<Item = Result<bytes::Bytes, SavantError>> + Send + Unpin>, SavantError> {
        let response = self.client.post(format!("{}/chat/completions", self.url))
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("LM Studio request failed: {}", e)))?;

        let stream = response.bytes_stream().map(|res| {
            res.map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))
        });

        Ok(Box::new(stream))
    }
}

/// Groq implementation (OpenAI-compatible).
pub struct GroqProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
}

#[async_trait]
impl LlmProvider for GroqProvider {
    async fn stream_completion(&self, messages: Vec<serde_json::Value>) -> Result<Box<dyn Stream<Item = Result<bytes::Bytes, SavantError>> + Send + Unpin>, SavantError> {
        let response = self.client.post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Groq request failed: {}", e)))?;

        let stream = response.bytes_stream().map(|res| {
            res.map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))
        });

        Ok(Box::new(stream))
    }
}

/// Perplexity implementation (OpenAI-compatible).
pub struct PerplexityProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
}

#[async_trait]
impl LlmProvider for PerplexityProvider {
    async fn stream_completion(&self, messages: Vec<serde_json::Value>) -> Result<Box<dyn Stream<Item = Result<bytes::Bytes, SavantError>> + Send + Unpin>, SavantError> {
        let response = self.client.post("https://api.perplexity.ai/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Perplexity request failed: {}", e)))?;

        let stream = response.bytes_stream().map(|res| {
            res.map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))
        });

        Ok(Box::new(stream))
    }
}

/// OpenRouter implementation of LlmProvider (OpenAI-compatible).
pub struct OpenRouterProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    async fn stream_completion(&self, messages: Vec<serde_json::Value>) -> Result<Box<dyn Stream<Item = Result<bytes::Bytes, SavantError>> + Send + Unpin>, SavantError> {
        let response = self.client.post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/Savant-AI/Savant")
            .header("X-Title", "Savant Framework")
            .header("content-type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("OpenRouter request failed: {}", e)))?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(SavantError::AuthError(format!("OpenRouter API error: {}", err_text)));
        }

        let stream = response.bytes_stream().map(|res| {
            res.map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))
        });

        Ok(Box::new(stream))
    }
}
/// A decorator that adds retry logic to any LlmProvider.
pub struct RetryProvider {
    pub inner: Box<dyn LlmProvider>,
    pub max_retries: u32,
}

#[async_trait]
impl LlmProvider for RetryProvider {
    async fn stream_completion(&self, messages: Vec<serde_json::Value>) -> Result<Box<dyn Stream<Item = Result<bytes::Bytes, SavantError>> + Send + Unpin>, SavantError> {
        let mut attempts = 0;
        let mut last_error = SavantError::Unknown("Retry failed".to_string());

        while attempts < self.max_retries {
            match self.inner.stream_completion(messages.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    attempts += 1;
                    tracing::warn!("LLM provider attempt {} failed: {}. Retrying...", attempts, e);
                    last_error = e;
                    tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempts as u64)).await;
                }
            }
        }

        Err(last_error)
    }
}
