use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use tracing::{info, warn};

/// Protocol S-ATLAS: Autonomous Lesson Distillation (ALD)
/// Distills raw learnings into core sovereign SOUL files.
pub struct ALDEngine {
    workspace_root: PathBuf,
}

/// Identity evolution signal detected by ALD
#[derive(Debug, Clone)]
pub struct IdentitySignal {
    pub content: String,
    pub signal_type: String, // "identity", "trait", "mutation"
    pub confidence: f32,
}

impl ALDEngine {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Returns the new watermark, burst detection, and any identity evolution signals
    pub fn distill(&self, watermark: u64) -> Result<(u64, bool, Vec<IdentitySignal>), Box<dyn std::error::Error>> {
        let learnings_path = self.workspace_root.join("LEARNINGS.md");
        if !learnings_path.exists() {
            return Ok((0, false, Vec::new()));
        }

        let mut file = fs::File::open(&learnings_path)?;
        let file_len = file.metadata()?.len();

        let actual_offset = if watermark > file_len {
            warn!("ALD: Watermark exceeds file length. Resetting to 0.");
            0
        } else {
            watermark
        };
        file.seek(SeekFrom::Start(actual_offset))?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;

        if content.is_empty() {
            return Ok((file_len, false, Vec::new()));
        }

        let blocks: Vec<&str> = content.split("### Learning").collect();
        let mut priority_hits = 0;
        let mut identity_signals = Vec::new();

        let start_idx = if actual_offset == 0 { 1 } else { 0 };
        for block in blocks.iter().skip(start_idx) {
            let is_strategic = block.contains("[STRATEGY]")
                || block.contains("[STRATEGIC]")
                || block.contains("Strategic Insight");
            let is_engineering =
                block.contains("[ENGINEERING]") || block.contains("Protocol Precision");

            if is_strategic {
                // Route strategic blocks through mutation proposal flow instead of direct write
                let signal = IdentitySignal {
                    content: block.trim().to_string(),
                    signal_type: "strategic".to_string(),
                    confidence: 0.85,
                };
                identity_signals.push(signal);
                priority_hits += 2;
            } else if is_engineering {
                self.promote_to_agents(block)?;
                priority_hits += 1;
            }

            // Detect identity-related blocks and emit as signals (not direct writes)
            let is_identity = block.contains("[IDENTITY]")
                || block.contains("[TRAIT]")
                || block.contains("[MUTATION]");
            if is_identity {
                let signal_type = if block.contains("[IDENTITY]") {
                    "identity"
                } else if block.contains("[TRAIT]") {
                    "trait"
                } else {
                    "mutation"
                };
                let signal = IdentitySignal {
                    content: block.trim().to_string(),
                    signal_type: signal_type.to_string(),
                    confidence: 0.90,
                };
                identity_signals.push(signal);
                info!("ALD: Identity evolution signal detected (type: {})", signal_type);
                priority_hits += 1;
            }
        }

        let burst_detected = priority_hits >= 3;
        Ok((file_len, burst_detected, identity_signals))
    }

    /// Process identity signals by routing them through the mutation flow.
    /// Takes a Nexus reference to emit SoulMutationPropose events.
    pub fn process_identity_signal(
        &self,
        signal: &IdentitySignal,
        agent_id: &str,
        nexus: &savant_core::bus::NexusBridge,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mutation_type = match signal.signal_type.as_str() {
            "identity" => "additive",
            "trait" => "transformative",
            "mutation" => "additive",
            _ => "additive",
        };

        let target_section = match signal.signal_type.as_str() {
            "identity" => "THE IDENTITY",
            "trait" => "OCEAN TRAITS",
            _ => "EVOLUTION",
        };

        info!(
            "[ALD] Emitting SoulMutationPropose for agent {} (type: {}, section: {})",
            agent_id, mutation_type, target_section
        );

        // Read current SOUL.md content for the before/after diff
        let soul_path = self.workspace_root.join("SOUL.md");
        let before_content = if soul_path.exists() {
            fs::read_to_string(&soul_path).unwrap_or_default()
        } else {
            String::new()
        };

        let mutation = serde_json::json!({
            "status": "pending",
            "mutation_id": uuid::Uuid::new_v4().to_string(),
            "agent_id": agent_id,
            "mutation_type": mutation_type,
            "target_section": target_section,
            "proposed_content": signal.content,
            "before_content": before_content,
            "reasoning": format!("ALD-distilled identity signal (confidence: {:.2})", signal.confidence),
            "conversations_triggered": [],
            "confidence": signal.confidence,
            "proposed_at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64,
            "decided_at": serde_json::Value::Null,
            "source_evidence": [],
            "before_hash": format!("{:x}", { use sha2::{Digest, Sha256}; Sha256::digest(before_content.as_bytes()) }),
        });

        // Persist to EVOLUTION.jsonl
        let evo_path = self.workspace_root.join("EVOLUTION.jsonl");
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&evo_path) {
            let line = serde_json::to_string(&mutation).unwrap_or_default();
            let _ = writeln!(file, "{}", line);
        }

        // Emit to Nexus for real-time dashboard update
        drop(nexus.publish(
            "system.evolution.mutation_proposed",
            &serde_json::to_string(&mutation).unwrap_or_default(),
        ));

        Ok(())
    }


    fn promote_to_agents(&self, _block: &str) -> Result<(), Box<dyn std::error::Error>> {
        // S-ATLAS distillation to AGENTS.md disabled — auto-generated artifacts
        // were polluting the system prompt with diary/identity content.
        // Distillations still flow to SOUL.md via promote_to_soul().
        Ok(())
    }
}
