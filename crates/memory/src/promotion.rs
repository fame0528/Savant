//! Personality-Driven Memory Promotion
//!
//! Background worker that scans memories, applies OCEAN trait-based decay factors,
//! and promotes high-value memories to canonical storage.
//!
//! # Mechanics
//! - `Conscientiousness` scalar: slows decay for security/constraint memories
//! - `Openness` scalar: lowers entropy threshold for exploratory observations
//! - Memories are scored based on: hit_count, age, entropy, importance, personality fit
//! - High-score memories promote to "canonical" category
//! - Low-score memories are archived

use serde::{Deserialize, Serialize};

pub use savant_core::types::PersonalityDelta;

/// OCEAN personality traits from agent SOUL.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityTraits {
    pub openness: f32,          // 0.0 - 1.0
    pub conscientiousness: f32, // 0.0 - 1.0
    pub extraversion: f32,      // 0.0 - 1.0
    pub agreeableness: f32,     // 0.0 - 1.0
    pub neuroticism: f32,       // 0.0 - 1.0
}

impl Default for PersonalityTraits {
    fn default() -> Self {
        Self {
            openness: 0.5,
            conscientiousness: 0.5,
            extraversion: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.5,
        }
    }
}

/// Memory promotion metrics used for scoring.
#[derive(Debug, Clone)]
pub struct PromotionMetrics {
    pub hit_count: u32,
    pub age_hours: f32,
    pub shannon_entropy: f32,
    pub importance: u8,
    pub category: String,
}

/// Promotion scoring engine.
pub struct PromotionEngine {
    personality: PersonalityTraits,
    /// Minimum score for promotion to canonical
    pub promotion_threshold: f32,
    /// Maximum age in hours before aggressive decay
    pub decay_after_hours: f32,
    /// Maximum allowed Euclidean distance from baseline OCEAN before auto-block
    pub personality_drift_limit: f32,
    /// Original baseline personality for drift comparison
    pub baseline_personality: Option<PersonalityTraits>,
    /// Running evolution score (0.0-1.0) updated each promotion cycle
    pub evolution_score: f32,
}

impl PromotionEngine {
    /// Creates a new promotion engine with the given personality traits.
    pub fn new(personality: PersonalityTraits) -> Self {
        Self {
            personality: personality.clone(),
            promotion_threshold: 0.7,
            decay_after_hours: 168.0,
            personality_drift_limit: 0.15,
            baseline_personality: Some(personality),
            evolution_score: 0.0,
        }
    }

    /// Updates the active personality traits for promotion scoring.
    ///
    /// The baseline personality (set at construction) is preserved for drift detection.
    /// Only the active scoring personality is updated.
    pub fn update_traits(&mut self, traits: PersonalityTraits) {
        self.personality = traits;
    }

    /// Updates the evolution score based on the latest promotion cycle results.
    ///
    /// The evolution score represents the ratio of high-value memories to total memories,
    /// providing a health metric for the memory system. A higher score indicates
    /// a greater proportion of valuable, frequently-accessed memories.
    pub fn update_evolution_score(&mut self, score: f32) {
        // Smooth the evolution score with exponential moving average (alpha=0.3)
        // to prevent wild swings from single-cycle anomalies
        let alpha = 0.3;
        self.evolution_score = alpha * score + (1.0 - alpha) * self.evolution_score;
        self.evolution_score = self.evolution_score.clamp(0.0, 1.0);
    }

    /// Calculates the promotion score for a memory.
    ///
    /// Score is 0.0 - 1.0. Higher = more likely to be promoted.
    ///
    /// Factors:
    /// - Hit count (access frequency)
    /// - Age decay (older memories score lower unless frequently accessed)
    /// - Shannon entropy (lower entropy = more deterministic = higher score)
    /// - Importance (direct multiplier)
    /// - Personality adjustment (Conscientiousness slows decay for security memories)
    pub fn calculate_score(&self, metrics: &PromotionMetrics) -> f32 {
        let mut score = 0.0;

        // Hit count contribution (0.0 - 0.3)
        let hit_score = (metrics.hit_count as f32 / 100.0).min(0.3);
        score += hit_score;

        // Age decay (0.0 - 0.3 penalty)
        let age_decay = if metrics.age_hours > self.decay_after_hours {
            ((metrics.age_hours - self.decay_after_hours) / self.decay_after_hours).min(0.3)
        } else {
            0.0
        };
        score -= age_decay;

        // Entropy bonus (lower entropy = more deterministic = higher score)
        let entropy_score = (1.0 - metrics.shannon_entropy).max(0.0) * 0.2;
        score += entropy_score;

        // Importance multiplier (1-10 scale)
        let importance_factor = metrics.importance as f32 / 10.0;
        score *= 1.0 + importance_factor;

        // Personality adjustment
        // High conscientiousness: slow decay for security/constraint memories
        if metrics.category.contains("security") || metrics.category.contains("config") {
            let conscientiousness_bonus = self.personality.conscientiousness * 0.2;
            score += conscientiousness_bonus;
        }

        // High openness: boost exploratory/observation memories
        if metrics.category.contains("observation") || metrics.category.contains("exploration") {
            let openness_bonus = self.personality.openness * 0.15;
            score += openness_bonus;
        }

        // High extraversion: boost social/collaboration/communication memories
        if metrics.category.contains("social")
            || metrics.category.contains("collaboration")
            || metrics.category.contains("communication")
        {
            let extraversion_bonus = self.personality.extraversion * 0.2;
            score += extraversion_bonus;
        }

        // High agreeableness: boost consensus/harmony/agreement memories
        if metrics.category.contains("consensus")
            || metrics.category.contains("harmony")
            || metrics.category.contains("agreement")
        {
            let agreeableness_bonus = self.personality.agreeableness * 0.15;
            score += agreeableness_bonus;
        }

        // High neuroticism: conservative decay for threat/error memories (hyper-vigilance)
        if (metrics.category.contains("threat") || metrics.category.contains("error"))
            && self.personality.neuroticism > 0.6
        {
            // Conservative: keep threat memories longer
            score += 0.1;
        }

        score.clamp(0.0, 1.0)
    }

    /// Determines if a memory should be promoted to canonical.
    pub fn should_promote(&self, metrics: &PromotionMetrics) -> bool {
        self.calculate_score(metrics) >= self.promotion_threshold
    }

    /// Determines if a memory should be archived.
    pub fn should_archive(&self, metrics: &PromotionMetrics) -> bool {
        self.calculate_score(metrics) < 0.2 && metrics.age_hours > self.decay_after_hours
    }

    /// Checks if a learning should be promoted to agent identity (SOUL.md mutation).
    /// Requires 5+ recurrences AND high significance (≥7) AND personality alignment.
    pub fn should_promote_to_identity(&self, metrics: &PromotionMetrics, recurrence_count: usize) -> bool {
        recurrence_count >= 5
            && metrics.importance >= 7
            && self.calculate_score(metrics) >= self.promotion_threshold
    }

    /// Checks whether a proposed personality delta stays within the drift limit.
    /// Returns Ok(()) if within bounds, Err with distance if exceeded.
    pub fn check_drift_guard(&self, delta: &PersonalityDelta) -> Result<(), f32> {
        let distance = delta.euclidean_distance();
        if distance > self.personality_drift_limit {
            Err(distance)
        } else {
            Ok(())
        }
    }

    /// Computes Euclidean distance from the baseline personality.
    pub fn distance_from_baseline(&self) -> f32 {
        let baseline = match &self.baseline_personality {
            Some(b) => b,
            None => return 0.0,
        };
        let p = &self.personality;
        ((p.openness - baseline.openness).powi(2)
            + (p.conscientiousness - baseline.conscientiousness).powi(2)
            + (p.extraversion - baseline.extraversion).powi(2)
            + (p.agreeableness - baseline.agreeableness).powi(2)
            + (p.neuroticism - baseline.neuroticism).powi(2))
        .sqrt()
    }
}

impl Default for PromotionEngine {
    fn default() -> Self {
        Self::new(PersonalityTraits::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_traits() {
        let traits = PersonalityTraits::default();
        assert_eq!(traits.openness, 0.5);
        assert_eq!(traits.conscientiousness, 0.5);
    }

    #[test]
    fn test_promotion_score_high_importance() {
        let engine = PromotionEngine::default();
        let metrics = PromotionMetrics {
            hit_count: 50,
            age_hours: 24.0,
            shannon_entropy: 0.3,
            importance: 9,
            category: "fact".to_string(),
        };
        let score = engine.calculate_score(&metrics);
        assert!(score > 0.5);
    }

    #[test]
    fn test_promotion_score_low_importance() {
        let engine = PromotionEngine::default();
        let metrics = PromotionMetrics {
            hit_count: 1,
            age_hours: 200.0,
            shannon_entropy: 0.9,
            importance: 2,
            category: "observation".to_string(),
        };
        let score = engine.calculate_score(&metrics);
        assert!(score < 0.5);
    }

    #[test]
    fn test_conscientiousness_bonus() {
        let traits = PersonalityTraits {
            conscientiousness: 1.0,
            ..Default::default()
        };
        let engine = PromotionEngine::new(traits);

        let security_metrics = PromotionMetrics {
            hit_count: 10,
            age_hours: 24.0,
            shannon_entropy: 0.5,
            importance: 5,
            category: "security".to_string(),
        };

        let neutral_metrics = PromotionMetrics {
            hit_count: 10,
            age_hours: 24.0,
            shannon_entropy: 0.5,
            importance: 5,
            category: "general".to_string(),
        };

        let security_score = engine.calculate_score(&security_metrics);
        let neutral_score = engine.calculate_score(&neutral_metrics);
        assert!(security_score > neutral_score);
    }

    #[test]
    fn test_openness_bonus() {
        let traits = PersonalityTraits {
            openness: 1.0,
            ..Default::default()
        };
        let engine = PromotionEngine::new(traits);

        let exploration_metrics = PromotionMetrics {
            hit_count: 5,
            age_hours: 24.0,
            shannon_entropy: 0.5,
            importance: 5,
            category: "observation".to_string(),
        };

        let fact_metrics = PromotionMetrics {
            hit_count: 5,
            age_hours: 24.0,
            shannon_entropy: 0.5,
            importance: 5,
            category: "fact".to_string(),
        };

        let explore_score = engine.calculate_score(&exploration_metrics);
        let fact_score = engine.calculate_score(&fact_metrics);
        assert!(explore_score > fact_score);
    }

    #[test]
    fn test_should_promote_threshold() {
        let engine = PromotionEngine::default();
        let high_metrics = PromotionMetrics {
            hit_count: 100,
            age_hours: 24.0,
            shannon_entropy: 0.1,
            importance: 9,
            category: "fact".to_string(),
        };
        assert!(engine.should_promote(&high_metrics));
    }

    #[test]
    fn test_should_archive_old_low_value() {
        let engine = PromotionEngine::default();
        let old_metrics = PromotionMetrics {
            hit_count: 0,
            age_hours: 500.0,
            shannon_entropy: 0.95,
            importance: 1,
            category: "observation".to_string(),
        };
        assert!(engine.should_archive(&old_metrics));
    }
}
