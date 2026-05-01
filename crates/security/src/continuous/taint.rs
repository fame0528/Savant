//! Taint Tracing — Tracks data provenance through the system.
//!
//! All external data ingestion is tagged with taint metadata.
//! During memory consolidation and dreaming, taint provenance is traced.
//! Heavily tainted memories require human-in-the-loop verification.

use serde::{Deserialize, Serialize};

/// Taint tag for tracking data provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaintTag {
    /// Source of the data.
    pub source: String,
    /// Timestamp of ingestion.
    pub timestamp: i64,
    /// Trust level (0.0 = untrusted, 1.0 = fully trusted).
    pub trust_level: f32,
    /// Chain of transformations applied to this data.
    pub provenance_chain: Vec<String>,
}

impl TaintTag {
    /// Creates a new taint tag from a source with the given trust level.
    pub fn new(source: &str, trust_level: f32) -> Self {
        Self {
            source: source.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            trust_level,
            provenance_chain: vec![source.to_string()],
        }
    }

    /// External web data — low trust.
    pub fn external_web() -> Self {
        Self::new("external_web", 0.2)
    }

    /// User-provided file — medium trust.
    pub fn user_file() -> Self {
        Self::new("user_file", 0.5)
    }

    /// System-generated — full trust.
    pub fn system() -> Self {
        Self::new("system", 1.0)
    }

    /// NREM replay output — medium-high trust (grounded in real memories).
    pub fn nrem_replay() -> Self {
        Self {
            source: "nrem_replay".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            trust_level: 0.7,
            provenance_chain: vec![
                "memory_replay".to_string(),
                "nrem_consolidation".to_string(),
            ],
        }
    }

    /// Dream engine output — medium trust (speculative exploration).
    pub fn dream() -> Self {
        Self {
            source: "dream".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            trust_level: 0.5,
            provenance_chain: vec![
                "latent_exploration".to_string(),
                "cross_domain_recombination".to_string(),
            ],
        }
    }

    /// Adds a transformation step to the provenance chain.
    pub fn add_transformation(&mut self, step: &str) {
        self.provenance_chain.push(step.to_string());
    }

    /// Compounds two taint tags (used during memory consolidation).
    /// Trust level becomes the minimum of the two sources.
    pub fn compound(&self, other: &TaintTag) -> Self {
        Self {
            source: format!("{}+{}", self.source, other.source),
            timestamp: chrono::Utc::now().timestamp(),
            trust_level: self.trust_level.min(other.trust_level),
            provenance_chain: {
                let mut chain = self.provenance_chain.clone();
                chain.extend(other.provenance_chain.clone());
                chain
            },
        }
    }

    /// Returns true if this data requires human-in-the-loop verification.
    pub fn requires_human_verification(&self) -> bool {
        self.trust_level < 0.3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_web_low_trust() {
        let tag = TaintTag::external_web();
        assert!(tag.trust_level < 0.3);
        assert!(tag.requires_human_verification());
    }

    #[test]
    fn test_system_full_trust() {
        let tag = TaintTag::system();
        assert_eq!(tag.trust_level, 1.0);
        assert!(!tag.requires_human_verification());
    }

    #[test]
    fn test_compound_takes_minimum() {
        let a = TaintTag::external_web();
        let b = TaintTag::system();
        let compounded = a.compound(&b);
        assert_eq!(compounded.trust_level, 0.2);
    }

    #[test]
    fn test_provenance_chain_grows() {
        let mut tag = TaintTag::external_web();
        tag.add_transformation("distillation");
        assert_eq!(tag.provenance_chain.len(), 2);
    }

    #[test]
    fn test_nrem_replay_trust() {
        let tag = TaintTag::nrem_replay();
        assert_eq!(tag.trust_level, 0.7);
        assert!(!tag.requires_human_verification());
    }

    #[test]
    fn test_dream_trust() {
        let tag = TaintTag::dream();
        assert_eq!(tag.trust_level, 0.5);
        assert!(!tag.requires_human_verification());
    }
}
