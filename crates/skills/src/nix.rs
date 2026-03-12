use savant_core::traits::SkillExecutor;
use savant_core::error::SavantError;
use std::pin::Pin;
use futures::future::Future;
use std::process::Command;

/// Executes skills within a nix-shell or nix flake environment for absolute reproducibility.
pub struct NixSkillExecutor {
    pub flake_path: String,
}

impl SkillExecutor for NixSkillExecutor {
    fn execute(&self, payload: &str) -> Pin<Box<dyn Future<Output = Result<String, SavantError>> + Send>> {
        let flake = self.flake_path.clone();
        let input = payload.to_string();

        Box::pin(async move {
            info!("Executing Nix Skill via flake: {}", flake);
            // Example: nix run .#skill_name -- "payload"
            let output = Command::new("nix")
                .args(["run", &flake, "--", &input])
                .output()
                .map_err(|e| SavantError::Unknown(format!("Nix execution failed: {}", e)))?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(SavantError::Unknown(String::from_utf8_lossy(&output.stderr).to_string()))
            }
        })
    }
}

use tracing::info;
