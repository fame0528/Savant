pub mod context_gatherer;
pub mod perception;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::PathBuf;
use tracing::debug;

/// A single prior heartbeat thought, stored for continuity across pulses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentThought {
    pub timestamp: i64,
    pub content: String,
}

/// Protocol C-ATLAS: Proactive Session-State WAL
/// Ensures zero-latency recovery of agent decisions and preferences.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WorkingBuffer {
    pub current_goal: String,
    pub context_summary: String,
    pub pending_actions: Vec<String>,
    pub recent_corrections: Vec<String>,
    pub agent_preferences: std::collections::HashMap<String, String>,
    pub last_pulse_hash: Option<u64>,
    pub current_lens_index: usize,
    pub last_reflection_hashes: Vec<u64>,
    pub ald_watermark: u64,
    #[serde(default)]
    pub recent_thoughts: Vec<RecentThought>,
    #[serde(default)]
    pub last_reflection_time: Option<i64>,
    /// Pending evolution mutation proposals awaiting user review
    #[serde(default)]
    pub pending_mutations: Vec<String>,
    /// Current OCEAN personality snapshot
    #[serde(default)]
    pub current_personality_o: f32,
    #[serde(default)]
    pub current_personality_c: f32,
    #[serde(default)]
    pub current_personality_e: f32,
    #[serde(default)]
    pub current_personality_a: f32,
    #[serde(default)]
    pub current_personality_n: f32,
    /// Growth metrics for evolution tracking
    #[serde(default)]
    pub total_pulses: u64,
    #[serde(default)]
    pub total_learnings: u64,
    #[serde(default)]
    pub self_modification_count: u64,
}

pub struct ProactivePartner {
    state_path: PathBuf,
    context_path: PathBuf,
}

impl ProactivePartner {
    pub fn new(root: PathBuf, config: &savant_core::config::ProactiveConfig) -> Self {
        Self {
            state_path: root.join(&config.session_state_file),
            context_path: root.join(&config.workspace_context_file),
        }
    }

    /// Commit the current working buffer to the sovereign WAL.
    pub fn commit_state(&self, buffer: &WorkingBuffer) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string(buffer)?;
        let formatted = format!("--- \n# Sovereign Session State (WAL)\n---\n\n{}", content);
        fs::write(&self.state_path, formatted)?;
        debug!("C-ATLAS: Session-State committed to WAL.");
        Ok(())
    }

    /// Materialize the working buffer from the WAL.
    pub fn restore_state(&self) -> Result<WorkingBuffer, Box<dyn std::error::Error>> {
        if !self.state_path.exists() {
            return Ok(WorkingBuffer::default());
        }
        let content = fs::read_to_string(&self.state_path)?;

        // AAA: Robust JSON extraction from Markdown-wrapped WAL
        let json_start = content
            .find('{')
            .ok_or("Invalid WAL: No JSON start found")?;
        let json_end = content.rfind('}').ok_or("Invalid WAL: No JSON end found")?;
        let json_str = &content[json_start..=json_end];

        let buffer: WorkingBuffer = serde_json::from_str(json_str)?;
        Ok(buffer)
    }

    /// OMEGA-VIII: Distill raw cognition into Layer 2 Workspace Context.
    pub fn distill_context(&self, summary: &str) -> Result<(), Box<dyn std::error::Error>> {
        let formatted = format!(
            "# 🧠 Sovereign Workspace Context (Layer 2)\n\n\
             > **Last Distillation:** {}\n\n\
             ## Current Knowledge Synthesis\n{}\n\n\
             --- \n\
             *Autonomous distillation performed by Savant Pulse.*",
            chrono::Utc::now().to_rfc3339(),
            summary
        );
        fs::write(&self.context_path, formatted)?;
        Ok(())
    }
}
