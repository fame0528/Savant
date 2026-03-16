use super::ToolExecutor;
use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::types::CapabilityGrants;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn};

/// Legacy executor for native scripts (bash/python).
/// Uses Landlock on Linux to restrict filesystem access to the workspace.
pub struct LegacyNativeExecutor {
    script_path: String,
    workspace_dir: PathBuf,
    capabilities: CapabilityGrants,
}

impl LegacyNativeExecutor {
    /// Creates a new LegacyNativeExecutor.
    pub fn new(
        script_path: String,
        workspace_dir: PathBuf,
        capabilities: CapabilityGrants,
    ) -> Self {
        Self {
            script_path,
            workspace_dir,
            capabilities,
        }
    }

    /// Applies platform-specific sandboxing to the command.
    #[cfg(target_os = "linux")]
    fn apply_sandbox(&self, cmd: &mut Command) {
        use caps::CapSet;
        use landlock::{AccessFs, PathBeneath, Ruleset, ABI};

        let workspace = self.workspace_dir.clone();

        // Safety: pre_exec is only called in the child process.
        // We use it to apply Landlock and drop capabilities before the script runs.
        unsafe {
            cmd.pre_exec(move || {
                // 1. Drop Capabilities (prevent privilege escalation)
                let _ = caps::clear(None, CapSet::Effective);

                // 2. Apply Landlock (filesystem restriction)
                let abi = ABI::V1;
                let access = AccessFs::from_all(abi);
                let ruleset = Ruleset::new()
                    .handle_access(access)
                    .ok()
                    .and_then(|r| r.create().ok())
                    .and_then(|r| r.add_rule(PathBeneath::new(&workspace, access).ok()?).ok());

                if let Some(ruleset) = ruleset {
                    let _ = ruleset.restrict_self();
                }

                Ok(())
            });
        }
    }

    /// Fallback for non-Linux platforms where Landlock is unavailable.
    #[cfg(not(target_os = "linux"))]
    fn apply_sandbox(&self, _cmd: &mut Command) {
        warn!("OS-level sandboxing (Landlock) is not supported on this platform. Running legacy script with limited isolation.");
    }
}

#[async_trait]
impl ToolExecutor for LegacyNativeExecutor {
    async fn execute(&self, args: Value) -> Result<String, SavantError> {
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(&self.script_path);
            c
        } else {
            let mut c = Command::new("bash");
            c.arg(&self.script_path);
            c
        };

        // Pass arguments via environment variable for compatibility with OpenClaw skills
        cmd.env("TOOL_ARGS", args.to_string());

        // Pass specific required environment variables
        for env_var in &self.capabilities.requires_env {
            if let Ok(val) = std::env::var(env_var) {
                cmd.env(env_var, val);
            }
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.current_dir(&self.workspace_dir);

        self.apply_sandbox(&mut cmd);

        info!(
            "Native Sandbox: Executing {} in {}",
            self.script_path,
            self.workspace_dir.display()
        );

        let output = cmd.output().await.map_err(SavantError::IoError)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SavantError::Unknown(format!("Script failed: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
