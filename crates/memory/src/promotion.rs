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
}

impl PromotionEngine {
    /// Creates a new promotion engine with the given personality traits.
    pub fn new(personality: PersonalityTraits) -> Self {
        Self {
            personality,
            promotion_threshold: 0.7,
            decay_after_hours: 168.0, // 7 days
        }
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
