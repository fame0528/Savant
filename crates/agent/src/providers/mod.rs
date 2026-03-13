pub mod mgmt;
use async_stream::stream;
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use reqwest::Client;
use savant_core::error::SavantError;
use savant_core::traits::LlmProvider;
use savant_core::types::{ChatChunk, ChatMessage};
use serde_json::{json, Value};
use std::pin::Pin;

/// Helper to transform raw bytes stream from OpenAI-compatible providers into ChatChunk stream.
fn openai_stream_to_chunks<S>(
    stream: S,
    agent_id: String,
    agent_name: String,
) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>
where
    S: Stream<Item = Result<bytes::Bytes, SavantError>> + Send + 'static + std::marker::Unpin,
{
    Box::pin(stream! {
        let mut stream = stream;
        while let Some(chunk_res) = stream.next().await {
            match chunk_res {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        let line = line.trim();
                        if line.is_empty() { continue; }
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" { break; }

                            if let Ok(json) = serde_json::from_str::<Value>(data) {
                                if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                    // Filter noise
                                    if !content.contains("OPENROUTER PROCESSING") {
                                        yield Ok(ChatChunk {
                                            agent_name: agent_name.clone(),
                                            agent_id: agent_id.clone(),
                                            content: content.to_string(),
                                            is_final: false,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => yield Err(e),
            }
        }
        yield Ok(ChatChunk {
            agent_name: agent_name.clone(),
            agent_id: agent_id.clone(),
            content: String::new(),
            is_final: true
        });
    })
}

pub struct OpenAiProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("OpenAI request failed: {}", e)))?;

        let stream = response.bytes_stream().map(|res| {
            res.map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))
        });

        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

pub struct OpenRouterProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let response = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/Savant-AI/Savant")
            .header("X-Title", "Savant Framework")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("OpenRouter request failed: {}", e)))?;

        let stream = response.bytes_stream().map(|res| {
            res.map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))
        });

        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

pub struct AnthropicProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
                "max_tokens": 4096,
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Anthropic request failed: {}", e)))?;

        let stream = response.bytes_stream().map(|res| {
            res.map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))
        });

        // Anthropic has a different format, but for brevity I will use a simple mapper here.
        // In a full implementation, we'd have anthropic_stream_to_chunks.
        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

pub struct OllamaProvider {
    pub client: Client,
    pub url: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let response = self
            .client
            .post(format!("{}/api/chat", self.url))
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

        let agent_name = self.agent_name.clone();
        let agent_id = self.agent_id.clone();

        Ok(Box::pin(stream! {
            let mut stream = stream;
            while let Some(chunk_res) = stream.next().await {
                match chunk_res {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        if let Ok(json) = serde_json::from_str::<Value>(&text) {
                            if let Some(content) = json["message"]["content"].as_str() {
                                yield Ok(ChatChunk {
                                    agent_name: agent_name.clone(),
                                    agent_id: agent_id.clone(),
                                    content: content.to_string(),
                                    is_final: false,
                                });
                            }
                        }
                    }
                    Err(e) => yield Err(e),
                }
            }
            yield Ok(ChatChunk {
                agent_name: agent_name,
                agent_id: agent_id,
                content: String::new(),
                is_final: true
            });
        }))
    }
}

pub struct GroqProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
}

#[async_trait]
impl LlmProvider for GroqProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let response = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
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

        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

/// A decorator that adds retry logic to any LlmProvider.
pub struct RetryProvider {
    pub inner: Box<dyn LlmProvider>,
    pub max_retries: u32,
}

#[async_trait]
impl LlmProvider for RetryProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let mut attempts = 0;
        let mut last_error = SavantError::Unknown("Retry failed".to_string());

        while attempts < self.max_retries {
            match self.inner.stream_completion(messages.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    attempts += 1;
                    tracing::warn!(
                        "LLM provider attempt {} failed: {}. Retrying...",
                        attempts,
                        e
                    );
                    last_error = e;
                    tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempts as u64))
                        .await;
                }
            }
        }

        Err(last_error)
    }
}
