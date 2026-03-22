//! Free Model Router
//!
//! Manages model selection using only free models. Never uses paid models.
//!
//! Selection strategy:
//!   1. `openrouter/hunter-alpha` (primary) — 1M context window
//!   2. `openrouter/healer-alpha` (backup) — 256k context
//!   3. `stepfun/step-3.5-flash:free` (step 3) — 256k context
//!   4. `openrouter/free` (free router) — context window unknown
//!
//! **Rotation is very rare.** The primary model should almost always be used.
//! Rotation only happens when:
//!   - The endpoint is failing (error/timeout)
//!   - The user explicitly asks to switch via dashboard
//!
//! All model selection is dashboard-controllable via ConfigGet/ConfigSet.

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// The primary model — always tried first.
const PRIMARY_MODEL: &str = "openrouter/hunter-alpha";

/// The backup model — tried if primary fails.
const BACKUP_MODEL: &str = "openrouter/healer-alpha";

/// Step 3: specific free model.
const STEP3_MODEL: &str = "stepfun/step-3.5-flash:free";

/// Final fallback: OpenRouter free model router (picks best available free model automatically).
const FREE_ROUTER: &str = "openrouter/free";

/// All free models in priority order.
const ALL_FREE_MODELS: &[&str] = &[
    "openrouter/hunter-alpha",
    "openrouter/healer-alpha",
    "stepfun/step-3.5-flash:free",
    "openrouter/free",
];

/// Represents a model selection attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAttempt {
    pub model: String,
    pub attempt_number: u32,
    pub strategy: String,
}

/// Free model router — selects the best available free model.
pub struct FreeModelRouter;

impl FreeModelRouter {
    /// Returns the primary model (hunter-alpha).
    pub fn primary() -> &'static str {
        PRIMARY_MODEL
    }

    /// Returns the backup model (healer-alpha).
    pub fn backup() -> &'static str {
        BACKUP_MODEL
    }

    /// Returns all free models in priority order.
    pub fn all_free_models() -> &'static [&'static str] {
        ALL_FREE_MODELS
    }

    /// Selects the next model based on the rotation strategy.
    ///
    /// Strategy:
    ///   0 → primary (hunter-alpha)
    ///   1 → backup (healer-alpha)
    ///   2 → step 3 (stepfun/step-3.5-flash:free)
    ///   3+ → free router (openrouter/free picks best available)
    pub fn select_model(attempt: u32) -> ModelAttempt {
        let (model, strategy) = match attempt {
            0 => {
                info!("Model selection: attempt 0 → primary (hunter-alpha)");
                (PRIMARY_MODEL, "primary")
            }
            1 => {
                info!("Model selection: attempt 1 → backup (healer-alpha)");
                (BACKUP_MODEL, "backup")
            }
            2 => {
                info!("Model selection: attempt 2 → step3 (stepfun/step-3.5-flash:free)");
                (STEP3_MODEL, "step3")
            }
            n => {
                warn!(
                    "Model selection: attempt {} → free router (openrouter/free)",
                    n
                );
                (FREE_ROUTER, "free_router")
            }
        };

        ModelAttempt {
            model: model.to_string(),
            attempt_number: attempt,
            strategy: strategy.to_string(),
        }
    }

    /// Returns a list of all free models for the dashboard.
    pub fn dashboard_model_list() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                name: PRIMARY_MODEL.to_string(),
                display_name: "Hunter Alpha".to_string(),
                tier: "primary".to_string(),
                description: "Primary model. Fast, capable, free.".to_string(),
            },
            ModelInfo {
                name: BACKUP_MODEL.to_string(),
                display_name: "Healer Alpha".to_string(),
                tier: "backup".to_string(),
                description: "Backup model. Reliable, free.".to_string(),
            },
            ModelInfo {
                name: STEP3_MODEL.to_string(),
                display_name: "Step 3.5 Flash".to_string(),
                tier: "step3".to_string(),
                description: "Step 3 free model. Fast flash variant.".to_string(),
            },
            ModelInfo {
                name: FREE_ROUTER.to_string(),
                display_name: "OpenRouter Free Router".to_string(),
                tier: "free_router".to_string(),
                description: "OpenRouter picks the best available free model automatically."
                    .to_string(),
            },
        ]
    }

    /// Validates that a model name is a free model.
    pub fn is_free_model(model: &str) -> bool {
        ALL_FREE_MODELS.contains(&model)
    }
}

/// Model information for dashboard display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub display_name: String,
    pub tier: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primary_is_hunter_alpha() {
        assert_eq!(FreeModelRouter::primary(), "openrouter/hunter-alpha");
    }

    #[test]
    fn test_backup_is_healer_alpha() {
        assert_eq!(FreeModelRouter::backup(), "openrouter/healer-alpha");
    }

    #[test]
    fn test_select_model_attempt_0() {
        let attempt = FreeModelRouter::select_model(0);
        assert_eq!(attempt.model, "openrouter/hunter-alpha");
        assert_eq!(attempt.strategy, "primary");
    }

    #[test]
    fn test_select_model_attempt_1() {
        let attempt = FreeModelRouter::select_model(1);
        assert_eq!(attempt.model, "openrouter/healer-alpha");
        assert_eq!(attempt.strategy, "backup");
    }

    #[test]
    fn test_select_model_attempt_2() {
        let attempt = FreeModelRouter::select_model(2);
        assert_eq!(attempt.model, "stepfun/step-3.5-flash:free");
        assert_eq!(attempt.strategy, "step3");
    }

    #[test]
    fn test_select_model_attempt_3_plus_is_free_router() {
        for n in 3..10 {
            let attempt = FreeModelRouter::select_model(n);
            assert_eq!(attempt.model, "openrouter/free");
            assert_eq!(attempt.strategy, "free_router");
        }
    }

    #[test]
    fn test_all_free_models_no_paid() {
        let models = FreeModelRouter::all_free_models();
        for model in models {
            assert!(
                !model.contains("claude")
                    && !model.contains("gpt-4")
                    && !model.contains("gpt-5")
                    && !model.contains("opus")
                    && !model.contains("sonnet")
                    && !model.contains("grok"),
                "Found paid model in free list: {}",
                model
            );
        }
    }

    #[test]
    fn test_is_free_model() {
        assert!(FreeModelRouter::is_free_model("openrouter/hunter-alpha"));
        assert!(FreeModelRouter::is_free_model("openrouter/healer-alpha"));
        assert!(FreeModelRouter::is_free_model(
            "stepfun/step-3.5-flash:free"
        ));
        assert!(FreeModelRouter::is_free_model("openrouter/free"));
        assert!(!FreeModelRouter::is_free_model("anthropic/claude-opus"));
    }

    #[test]
    fn test_dashboard_model_list_count() {
        let models = FreeModelRouter::dashboard_model_list();
        assert_eq!(models.len(), 4);
    }
}
