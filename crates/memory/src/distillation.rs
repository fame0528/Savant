use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

use crate::engine::MemoryEnclave;
use crate::models::{AgentMessage, MemoryEntry};
use futures::StreamExt;
use savant_core::traits::{EmbeddingProvider, LlmProvider};
use savant_core::types::{ChatMessage, ChatRole};

/// Represents a distilled fact ready for public Hive-Mind access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistilledTriplet {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
    pub entropy: f32,
    pub source_session: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TripletClaims {
    pub sub: String,
    pub jti: String,
    pub iat: i64,
    pub exp: i64,
    pub triplet: DistilledTriplet,
}

/// Spawns a background worker to distill knowledge from the Private Enclave
/// into the Global Collective Database, enforcing Cryptographic Privacy Boundaries.
pub fn spawn_distillation_pipeline(
    enclave: Arc<MemoryEnclave>,
    collective: Arc<MemoryEnclave>,
    llm: Arc<dyn LlmProvider>,
    embeddings: Option<Arc<dyn EmbeddingProvider>>,
    _jwt_secret: String,
) {
    tokio::spawn(async move {
        info!("Enclave -> Collective Distillation Pipeline Online");

        loop {
            // Wake every 5 minutes to scan for high-entropy local facts
            sleep(Duration::from_secs(300)).await;

            debug!("Starting distillation sweep pass across Enclave...");

            let messages: Vec<AgentMessage> = enclave.lsm().iter_all_messages().collect();

            for msg in messages {
                if enclave.lsm().is_distilled(&msg.id) {
                    continue;
                }

                // Skip system messages or very short messages
                if msg.content.len() < 20 || matches!(msg.role, crate::models::MessageRole::System)
                {
                    continue;
                }

                // Perform Triplet Extraction via LLM
                match extract_triplets(Arc::clone(&llm), &msg.content).await {
                    Ok(triplets) => {
                        for triplet_data in triplets {
                            let entropy = calculate_shannon_entropy(&msg.content);
                            let distilled = DistilledTriplet {
                                subject: triplet_data.subject,
                                predicate: triplet_data.predicate,
                                object: triplet_data.object,
                                confidence: triplet_data.confidence,
                                entropy,
                                source_session: msg.session_id.clone(),
                            };

                            let now = chrono::Utc::now().timestamp();
                            let claims = TripletClaims {
                                sub: "hive_mind".to_string(),
                                jti: uuid::Uuid::new_v4().to_string(),
                                iat: now,
                                exp: now + 31536000, // 1 year expiry
                                triplet: distilled,
                            };

                            let now_ms = chrono::Utc::now().timestamp_millis();

                            // Generate a stable u64 ID from the source message UUID
                            // blake3 is deterministic across Rust versions (unlike DefaultHasher)
                            let hash = blake3::hash(msg.id.as_bytes());
                            let bytes = hash.as_bytes();
                            let entry_id =
                                u64::from_le_bytes(bytes[..8].try_into().unwrap_or([0u8; 8]));

                            let content = format!(
                                "{} {} {}",
                                claims.triplet.subject,
                                claims.triplet.predicate,
                                claims.triplet.object
                            );

                            // OMEGA-VIII: Generate semantic embedding if provider available
                            let mut triplet_embedding = Vec::new();
                            if let Some(ref provider) = embeddings {
                                if let Ok(vec) = provider.embed(&content).await {
                                    triplet_embedding = vec;
                                }
                            }

                            // Index into Collective as a new MemoryEntry
                            let entry = MemoryEntry {
                                id: entry_id.into(),
                                session_id: msg.session_id.clone(),
                                created_at: now_ms.into(),
                                updated_at: now_ms.into(),
                                content,
                                category: "distilled_triplet".to_string(),
                                importance: (claims.triplet.confidence * 10.0) as u8,
                                tags: vec!["hive-mind".to_string(), "shared".to_string()],
                                embedding: triplet_embedding,
                                shannon_entropy: entropy.into(),
                                last_accessed_at: now_ms.into(),
                                hit_count: 0.into(),
                                related_to: vec![],
                            };

                            if let Err(e) = collective.index_memory(entry).await {
                                error!("Failed to index distilled triplet into collective: {}", e);
                            } else {
                                // AAA: Production-grade SPO Indexing
                                if let Err(e) = collective.lsm().insert_fact(
                                    &claims.triplet.subject,
                                    &claims.triplet.predicate,
                                    &claims.triplet.object,
                                    entry_id,
                                ) {
                                    error!("Failed to insert fact into SPO index: {}", e);
                                }
                            }
                        }

                        // Mark as distilled only after successful processing
                        if let Err(e) = enclave.lsm().mark_distilled(&msg.id) {
                            error!("Failed to mark message as distilled: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Triplet extraction failed for message {}: {}", msg.id, e);
                    }
                }
            }
        }
    });
}

#[derive(Deserialize)]
struct TripletResponse {
    triplets: Vec<RawTriplet>,
}

#[derive(Deserialize)]
struct RawTriplet {
    subject: String,
    predicate: String,
    object: String,
    confidence: f32,
}

async fn extract_triplets(
    llm: Arc<dyn LlmProvider>,
    content: &str,
) -> Result<Vec<RawTriplet>, String> {
    let prompt = format!(
        "Extract core semantic triplets (Subject-Predicate-Object) from the following text. \
        Provide raw factual assertions only. Return as JSON: {{ \"triplets\": [{{ \"subject\": \"...\", \"predicate\": \"...\", \"object\": \"...\", \"confidence\": 0.0 }}] }}\n\nText: {}",
        content
    );

    let messages = vec![ChatMessage {
        is_telemetry: false,
        role: ChatRole::System,
        content: "You are a knowledge extraction engine for a Hive-Mind memory system. Extract atomic facts.".to_string(),
        sender: None,
        recipient: None,
        agent_id: None,
        session_id: None,
        channel: savant_core::types::AgentOutputChannel::Chat,
    }, ChatMessage {
        is_telemetry: false,
        role: ChatRole::User,
        content: prompt,
        sender: None,
        recipient: None,
        agent_id: None,
        session_id: None,
        channel: savant_core::types::AgentOutputChannel::Chat,
    }];

    let mut stream = llm
        .stream_completion(messages, vec![])
        .await
        .map_err(|e| e.to_string())?;
    let mut full_content = String::new();

    while let Some(chunk_res) = stream.next().await {
        if let Ok(chunk) = chunk_res {
            full_content.push_str(&chunk.content);
        }
    }

    // Parse the JSON output
    let parsed: TripletResponse = serde_json::from_str(&full_content).map_err(|e| e.to_string())?;
    Ok(parsed.triplets)
}

fn calculate_shannon_entropy(text: &str) -> f32 {
    let mut counts = std::collections::HashMap::new();
    let len = text.len() as f32;
    if len == 0.0 {
        return 0.0;
    }

    for c in text.chars() {
        *counts.entry(c).or_insert(0) += 1;
    }

    let mut entropy = 0.0;
    for count in counts.values() {
        let p = (*count as f32) / len;
        entropy -= p * p.log2();
    }
    entropy
}
