//
use savant_core::error::SavantError;
use async_trait::async_trait;
use std::process::Command;
use tracing::info;

/// Executes skills within a nix-shell or nix flake environment for absolute reproducibility.
pub struct NixSkillExecutor {
    pub flake_path: String,
}

#[async_trait]
impl savant_core::traits::Tool for NixSkillExecutor {
    fn name(&self) -> &str { "nix_skill" }
    fn description(&self) -> &str { "Executes a skill within a Nix environment." }
    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError> {
        let flake = self.flake_path.clone();
        let input = payload.to_string();

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
    }
}
