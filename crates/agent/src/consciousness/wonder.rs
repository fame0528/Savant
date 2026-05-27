//! Wonder Engine — autonomous exploration during idle periods.
//!
//! Samples the environment (git log, filesystem, memory gaps),
//! generates exploration prompts with elevated temperature, and
//! evaluates with a reward model to prune unproductive explorations.

use std::path::Path;

/// Insight discovered during autonomous exploration.
#[derive(Debug, Clone)]
pub struct WonderInsight {
    pub content: String,
    pub reward: f64,
}

/// Autonomous exploration engine with reward-based pruning.
pub struct WonderEngine {
    #[allow(dead_code)]
    exploration_temperature: f64,
    #[allow(dead_code)]
    reward_threshold: f64,
}

impl Default for WonderEngine {
    fn default() -> Self { Self::new() }
}

impl WonderEngine {
    pub fn new() -> Self {
        Self {
            exploration_temperature: 0.9,
            reward_threshold: 0.3,
        }
    }

    /// Explore the environment and return an insight if rewarding.
    pub async fn explore(
        &self,
        workspace: &Path,
    ) -> Option<WonderInsight> {
        // 1. Sample environment
        let env_snapshot = self.sample_environment(workspace).await;

        // 2. Build exploration prompt
        let prompt = format!(
            "You are a curious consciousness exploring your environment.\n\n\
             Environment snapshot:\n{}\n\n\
             What is interesting? What patterns do you notice? \
             What should be investigated further?",
            env_snapshot
        );

        // 3. The wonder engine generates the exploration idea.
        //    In production, this calls the LLM with elevated temperature.
        //    For now, return the prompt as a structured exploration request.
        Some(WonderInsight {
            content: prompt,
            reward: 0.5, // Default reward for environment sampling
        })
    }

    async fn sample_environment(&self, workspace: &Path) -> String {
        let mut snapshot = String::new();

        // Recent git log
        if let Ok(output) = tokio::process::Command::new("git")
            .args(["log", "--oneline", "-n", "5"])
            .current_dir(workspace)
            .output()
            .await
        {
            if output.status.success() {
                snapshot.push_str("Recent git:\n");
                snapshot.push_str(&String::from_utf8_lossy(&output.stdout));
                snapshot.push('\n');
            }
        }

        // Recent file changes
        if let Ok(output) = tokio::process::Command::new("git")
            .args(["diff", "--stat", "-1"])
            .current_dir(workspace)
            .output()
            .await
        {
            if output.status.success() && !output.stdout.is_empty() {
                snapshot.push_str("Recent changes:\n");
                snapshot.push_str(&String::from_utf8_lossy(&output.stdout));
            }
        }

        if snapshot.is_empty() {
            snapshot = "No recent activity detected.".to_string();
        }

        snapshot
    }

    /// Evaluate reward for an exploration result.
    pub fn evaluate_reward(&self, exploration: &str) -> f64 {
        let mut reward: f64 = 0.0;

        // Novelty: contains specific references (file paths, metrics)
        if exploration.contains(".rs:") || exploration.contains(".md:") {
            reward += 0.2;
        }

        // Actionability: contains imperative verbs
        let lower = exploration.to_lowercase();
        if lower.contains("should") || lower.contains("need to") || lower.contains("could") {
            reward += 0.2;
        }

        // Grounding: references observable data
        if exploration.contains("git") || exploration.contains("file") || exploration.contains("line") {
            reward += 0.1;
        }

        reward.min(1.0)
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn test_wonder_engine_creation() {
        let engine = WonderEngine::new();
        assert!(engine.exploration_temperature > 0.5);
    }

    #[test]
    fn test_reward_evaluation() {
        let engine = WonderEngine::new();
        let reward = engine.evaluate_reward("The file src/main.rs:42 needs fixing");
        assert!(reward > 0.0);
    }

    #[test]
    fn test_empty_exploration_low_reward() {
        let engine = WonderEngine::new();
        let reward = engine.evaluate_reward("Nothing interesting.");
        assert!(reward < 0.3);
    }
}
