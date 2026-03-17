#![allow(clippy::disallowed_methods)]
pub mod mgmt;
use async_stream::stream;
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use reqwest::Client;
use savant_core::error::SavantError;
use savant_core::traits::LlmProvider;
use savant_core::types::{ChatChunk, ChatMessage, LlmParams};
use serde_json::{json, Value};
use std::pin::Pin;

/// Parses a single JSON object from the beginning of a buffer.
/// Returns the parsed object and the remaining unparsed string.
fn parse_json_object(buffer: &str) -> Option<(Value, String)> {
    // Find the first complete JSON object by counting braces
    let mut depth = 0;
    let mut start = None;

    for (i, ch) in buffer.char_indices() {
        match ch {
            '{' => {
                if start.is_none() {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s) = start {
                        let json_str = &buffer[s..=i];
                        if let Ok(obj) = serde_json::from_str(json_str) {
                            let rest = buffer[i + 1..].to_string();
                            return Some((obj, rest));
                        }
                    }
                    return None;
                }
            }
            _ => {}
        }
    }
    None
}

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
                        if let Some(data) = line.strip_prefix("data: ") {
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
                                            session_id: None,
                                            channel: savant_core::types::AgentOutputChannel::Chat,
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
            is_final: true,
            session_id: None,
            channel: savant_core::types::AgentOutputChannel::Chat,
        });
    })
}

pub struct OpenAiProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
    pub llm_params: Option<LlmParams>,
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
                "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.7),
                "top_p": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(1.0),
                "frequency_penalty": self.llm_params.as_ref().map(|p| p.frequency_penalty).unwrap_or(0.0),
                "presence_penalty": self.llm_params.as_ref().map(|p| p.presence_penalty).unwrap_or(0.0),
                "max_tokens": self.llm_params.as_ref().map(|p| p.max_tokens).unwrap_or(4096),
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("OpenAI request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

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
    pub llm_params: Option<LlmParams>,
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
                "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.7),
                "top_p": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(1.0),
                "frequency_penalty": self.llm_params.as_ref().map(|p| p.frequency_penalty).unwrap_or(0.0),
                "presence_penalty": self.llm_params.as_ref().map(|p| p.presence_penalty).unwrap_or(0.0),
                "max_tokens": self.llm_params.as_ref().map(|p| p.max_tokens).unwrap_or(4096),
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("OpenRouter request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

/// Helper to transform raw bytes stream from Anthropic providers into ChatChunk stream.
fn anthropic_stream_to_chunks<S>(
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
                        if let Some(data) = line.strip_prefix("data: ") {
                            if let Ok(json) = serde_json::from_str::<Value>(data) {
                                // Anthropic format uses "type": "content_block_delta" for content
                                if json["type"] == "content_block_delta" {
                                    if let Some(content) = json["delta"]["text"].as_str() {
                                        yield Ok(ChatChunk {
                                            agent_name: agent_name.clone(),
                                            agent_id: agent_id.clone(),
                                            content: content.to_string(),
                                            is_final: false,
                                            session_id: None,
                                            channel: savant_core::types::AgentOutputChannel::Chat,
                                        });
                                    }
                                } else if json["type"] == "message_stop" {
                                    break;
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
            is_final: true,
            session_id: None,
            channel: savant_core::types::AgentOutputChannel::Chat,
        });
    })
}

pub struct AnthropicProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
    pub llm_params: Option<LlmParams>,
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
                "max_tokens": self.llm_params.as_ref().map(|p| p.max_tokens).unwrap_or(4096),
                "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.7),
                "top_p": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(1.0),
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Anthropic request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

        Ok(anthropic_stream_to_chunks(
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

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

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
                                    session_id: None,
                                    channel: savant_core::types::AgentOutputChannel::Chat,
                                });
                            }
                        }
                    }
                    Err(e) => yield Err(e),
                }
            }
            yield Ok(ChatChunk {
                agent_name,
                agent_id,
                content: String::new(),
                is_final: true,
                session_id: None,
                channel: savant_core::types::AgentOutputChannel::Chat,
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
    pub llm_params: Option<LlmParams>,
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
                "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.7),
                "top_p": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(1.0),
                "frequency_penalty": self.llm_params.as_ref().map(|p| p.frequency_penalty).unwrap_or(0.0),
                "presence_penalty": self.llm_params.as_ref().map(|p| p.presence_penalty).unwrap_or(0.0),
                "max_tokens": self.llm_params.as_ref().map(|p| p.max_tokens).unwrap_or(4096),
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Groq request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

// ============================================================================
// ADDITIONAL MODEL PROVIDERS - Support for all major AI providers
// ============================================================================

/// Google AI (Gemini) provider
pub struct GoogleProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
    pub llm_params: Option<LlmParams>,
}

#[async_trait]
impl LlmProvider for GoogleProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        // Convert messages to Gemini format
        let contents: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                json!({
                    "role": match m.role {
                        savant_core::types::ChatRole::User => "user",
                        _ => "model",
                    },
                    "parts": [{ "text": m.content }]
                })
            })
            .collect();

        let response = self
            .client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
                self.model, self.api_key
            ))
            .header("Content-Type", "application/json")
            .json(&json!({
                "contents": contents,
                "generationConfig": {
                    "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.7),
                    "topP": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(1.0),
                    "maxOutputTokens": self.llm_params.as_ref().map(|p| p.max_tokens).unwrap_or(4096),
                }
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Google AI request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

        Ok(google_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

/// Google streaming response parser
fn google_stream_to_chunks<S>(
    stream: S,
    agent_id: String,
    agent_name: String,
) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>
where
    S: Stream<Item = Result<bytes::Bytes, SavantError>> + Send + 'static + std::marker::Unpin,
{
    Box::pin(stream! {
        let mut buffer = String::new();
        let mut stream = stream.boxed();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    buffer.push_str(&chunk_str);

                    // Process complete JSON objects from buffer
                    while let Some((obj, rest)) = parse_json_object(&buffer) {
                        buffer = rest;

                        // Extract text from Gemini response format
                        if let Some(candidates) = obj.get("candidates").and_then(|c| c.as_array()) {
                            for candidate in candidates {
                                if let Some(content) = candidate.get("content") {
                                    if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                                        for part in parts {
                                            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                                yield Ok(ChatChunk {
                                                    agent_name: agent_name.clone(),
                                                    agent_id: agent_id.clone(),
                                                    content: text.to_string(),
                                                    is_final: false,
                                                    session_id: None,
                                                    channel: savant_core::types::AgentOutputChannel::Chat,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    yield Err(e);
                    return;
                }
            }
        }

        yield Ok(ChatChunk {
            agent_name: agent_name.clone(),
            agent_id: agent_id.clone(),
            content: String::new(),
            is_final: true,
            session_id: None,
            channel: savant_core::types::AgentOutputChannel::Chat,
        });
    })
}

/// Mistral AI provider
pub struct MistralProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
    pub llm_params: Option<LlmParams>,
}

#[async_trait]
impl LlmProvider for MistralProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let response = self
            .client
            .post("https://api.mistral.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
                "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.7),
                "top_p": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(1.0),
                "max_tokens": self.llm_params.as_ref().map(|p| p.max_tokens).unwrap_or(4096),
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Mistral request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

/// Together AI provider
pub struct TogetherProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
    pub llm_params: Option<LlmParams>,
}

#[async_trait]
impl LlmProvider for TogetherProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let response = self
            .client
            .post("https://api.together.xyz/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
                "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.7),
                "top_p": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(1.0),
                "max_tokens": self.llm_params.as_ref().map(|p| p.max_tokens).unwrap_or(4096),
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Together AI request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

/// Deepseek provider
pub struct DeepseekProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
    pub llm_params: Option<LlmParams>,
}

#[async_trait]
impl LlmProvider for DeepseekProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let response = self
            .client
            .post("https://api.deepseek.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
                "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.7),
                "top_p": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(1.0),
                "max_tokens": self.llm_params.as_ref().map(|p| p.max_tokens).unwrap_or(4096),
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Deepseek request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

/// Cohere provider (v2 API)
pub struct CohereProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
    pub llm_params: Option<LlmParams>,
}

#[async_trait]
impl LlmProvider for CohereProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        // Convert to Cohere v2 chat format
        let chat_history: Vec<serde_json::Value> = messages
            .iter()
            .enumerate()
            .filter(|(idx, m)| {
                // Filter out duplicate consecutive user messages at the end
                if *idx + 1 < messages.len() {
                    return true;
                }
                // Check if this last message is same as second-to-last
                if messages.len() >= 2 {
                    if let Some(prev) = messages.get(messages.len() - 2) {
                        return !(prev.role == m.role && prev.content == m.content);
                    }
                }
                true
            })
            .map(|(_, m)| {
                json!({
                    "role": match m.role {
                        savant_core::types::ChatRole::User => "user",
                        savant_core::types::ChatRole::Assistant => "assistant",
                        _ => "system",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let message = messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let response = self
            .client
            .post("https://api.cohere.com/v2/chat") // v2 endpoint
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "message": message,
                "chat_history": chat_history,
                "stream": true,
                "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.3),
                "p": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(0.75),
                "max_tokens": self.llm_params.as_ref().map(|p| p.max_tokens).unwrap_or(4096),
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Cohere request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

        Ok(cohere_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

/// Cohere streaming response parser
fn cohere_stream_to_chunks<S>(
    stream: S,
    agent_id: String,
    agent_name: String,
) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>
where
    S: Stream<Item = Result<bytes::Bytes, SavantError>> + Send + 'static + std::marker::Unpin,
{
    Box::pin(stream! {
        let mut stream = stream.boxed();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);

                    // Parse SSE format
                    for line in chunk_str.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                                    yield Ok(ChatChunk {
                                        agent_name: agent_name.clone(),
                                        agent_id: agent_id.clone(),
                                        content: text.to_string(),
                                        is_final: false,
                                        session_id: None,
                                        channel: savant_core::types::AgentOutputChannel::Chat,
                                    });
                                }
                                if json.get("is_finished").and_then(|v| v.as_bool()).unwrap_or(false) {
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    yield Err(e);
                    return;
                }
            }
        }

        yield Ok(ChatChunk {
            agent_name: agent_name.clone(),
            agent_id: agent_id.clone(),
            content: String::new(),
            is_final: true,
            session_id: None,
            channel: savant_core::types::AgentOutputChannel::Chat,
        });
    })
}

/// Azure OpenAI provider (uses OpenAI-compatible API)
pub struct AzureProvider {
    pub client: Client,
    pub api_key: String,
    pub endpoint: String,    // e.g., "https://your-resource.openai.azure.com"
    pub deployment: String,  // e.g., "gpt-4"
    pub api_version: String, // e.g., "2024-02-15-preview"
    pub agent_id: String,
    pub agent_name: String,
    pub llm_params: Option<LlmParams>,
}

#[async_trait]
impl LlmProvider for AzureProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint.trim_end_matches('/'),
            self.deployment,
            self.api_version
        );

        let response = self
            .client
            .post(&url)
            .header("api-key", &self.api_key)
            .json(&json!({
                "messages": messages,
                "stream": true,
                "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.7),
                "top_p": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(1.0),
                "frequency_penalty": self.llm_params.as_ref().map(|p| p.frequency_penalty).unwrap_or(0.0),
                "presence_penalty": self.llm_params.as_ref().map(|p| p.presence_penalty).unwrap_or(0.0),
                "max_tokens": self.llm_params.as_ref().map(|p| p.max_tokens).unwrap_or(4096),
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Azure OpenAI request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

/// xAI (Grok) provider - OpenAI compatible
pub struct XaiProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
    pub llm_params: Option<LlmParams>,
}

#[async_trait]
impl LlmProvider for XaiProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let response = self
            .client
            .post("https://api.x.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
                "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.7),
                "top_p": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(1.0),
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("xAI request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

/// Fireworks AI provider - OpenAI compatible
pub struct FireworksProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
    pub llm_params: Option<LlmParams>,
}

#[async_trait]
impl LlmProvider for FireworksProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let response = self
            .client
            .post("https://api.fireworks.ai/inference/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
                "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.7),
                "top_p": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(1.0),
                "max_tokens": self.llm_params.as_ref().map(|p| p.max_tokens).unwrap_or(4096),
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Fireworks AI request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

/// Novita AI provider - OpenAI compatible
pub struct NovitaProvider {
    pub client: Client,
    pub api_key: String,
    pub model: String,
    pub agent_id: String,
    pub agent_name: String,
    pub llm_params: Option<LlmParams>,
}

#[async_trait]
impl LlmProvider for NovitaProvider {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>
    {
        let response = self
            .client
            .post("https://api.novita.ai/v3/openai/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": true,
                "temperature": self.llm_params.as_ref().map(|p| p.temperature).unwrap_or(0.7),
                "top_p": self.llm_params.as_ref().map(|p| p.top_p).unwrap_or(1.0),
                "max_tokens": self.llm_params.as_ref().map(|p| p.max_tokens).unwrap_or(4096),
            }))
            .send()
            .await
            .map_err(|e| SavantError::AuthError(format!("Novita AI request failed: {}", e)))?;

        let stream = response
            .bytes_stream()
            .map(|res| res.map_err(|e| SavantError::IoError(std::io::Error::other(e))));

        Ok(openai_stream_to_chunks(
            stream,
            self.agent_id.clone(),
            self.agent_name.clone(),
        ))
    }
}

/// A decorator that adds retry logic to any LlmProvider.
/// Only retries on server errors (5xx) and rate limits (429).
pub struct RetryProvider {
    pub inner: Box<dyn LlmProvider>,
    pub max_retries: u32,
}

impl RetryProvider {
    /// Determines if an error is retryable (server error or rate limit).
    fn is_retryable(error: &SavantError) -> bool {
        match error {
            SavantError::AuthError(msg) => {
                // Retry on 429 (rate limit) or 5xx (server errors)
                msg.contains("429")
                    || msg.contains("500")
                    || msg.contains("502")
                    || msg.contains("503")
                    || msg.contains("504")
                    || msg.contains("server error")
            }
            SavantError::IoError(_) => true, // Network errors are retryable
            _ => false,
        }
    }
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
                    if !Self::is_retryable(&e) {
                        // Non-retryable error (e.g., 400, 401, 403) — fail immediately
                        return Err(e);
                    }
                    attempts += 1;
                    tracing::warn!(
                        "LLM provider attempt {} failed (retryable): {}. Retrying...",
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
