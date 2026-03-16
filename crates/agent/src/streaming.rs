use crate::react::AgentEvent;
use futures::stream::{Stream, StreamExt};
use savant_core::error::SavantError;
use savant_core::utils::parsing;
use serde_json::Value;

/// Parses JSON representations or Server-Sent Events from bytes to AgentEvents.
pub fn parse_llm_stream<S>(stream: S) -> impl Stream<Item = Result<AgentEvent, SavantError>>
where
    S: Stream<Item = Result<bytes::Bytes, SavantError>> + Unpin,
{
    use async_stream::stream;

    stream! {
        let mut stream = stream;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    let text = parsing::bytes_to_string(&bytes);

                    // Handle OpenRouter/OpenAI-style SSE
                    for line in text.lines() {
                        let line = line.trim();
                        if line.is_empty() { continue; }

                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                break;
                            }

                            if let Ok(json) = serde_json::from_str::<Value>(data) {
                                if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                    // Perfection Loop Refinement: Targeted exclusion of provider-level noise
                                    let noise = ["OPENROUTER PROCESSING", "PROVIDER:"];
                                    if !noise.iter().any(|&n| content.contains(n)) {
                                        yield Ok(AgentEvent::Thought(content.to_string()));
                                    }
                                }
                            }
                        } else if !line.is_empty() && !line.starts_with(':') {
                            // Support raw text with defensive JSON check
                            if serde_json::from_str::<Value>(line).is_err() {
                                yield Ok(AgentEvent::Thought(line.to_string() + "\n"));
                            }
                        }
                    }
                }
                Err(e) => {
                    yield Err(e);
                }
            }
        }
    }
}
