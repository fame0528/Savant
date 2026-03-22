use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::Tool;
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

/// SovereignShell: High-Fidelity Terminal Actuator
///
/// Unlike foundation.exec, SovereignShell is designed for complex, multi-stage
/// operations where stdout/stderr capture and exit status are critical for
/// autonomous error recovery.
pub struct SovereignShell;

impl Default for SovereignShell {
    fn default() -> Self {
        Self::new()
    }
}

impl SovereignShell {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SovereignShell {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute shell commands. Use for building, testing, installing packages, git operations, and system tasks."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute" },
                "cwd": { "type": "string", "description": "Working directory (optional)" }
            },
            "required": ["command"]
        })
    }

    fn requires_approval(&self) -> savant_core::traits::ApprovalRequirement {
        savant_core::traits::ApprovalRequirement::Conditional
    }

    fn domain(&self) -> savant_core::traits::ToolDomain {
        savant_core::traits::ToolDomain::Container
    }

    fn max_output_chars(&self) -> usize {
        10_000 // Shell output truncated to 10K chars (head+tail)
    }

    fn timeout_secs(&self) -> u64 {
        120 // Shell commands get 2 minutes
    }

    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let command = payload["command"].as_str().ok_or_else(|| {
            SavantError::Unknown("Missing 'command' field in shell payload".to_string())
        })?;

        // AAA: Sovereign Dialectic (Negotiated Consensus)
        // Detect destructive patterns
        let destructive_patterns = [
            "rm -rf",
            "format",
            "mkfs",
            "dd if=",
            "os.remove",
            "git reset --hard",
            "git clean -fd",
            "shred",
        ];

        for pattern in destructive_patterns {
            if command.contains(pattern) {
                let proposal = match pattern {
                    "rm -rf" => "Consider using 'mv' to a temporary directory instead.",
                    "git reset --hard" => "Consider 'git stash' to preserve current changes.",
                    _ => "Refine command to be non-destructive or request manual override.",
                };
                return Err(SavantError::ConsensusVeto(format!(
                    "Destructive command '{}' blocked by Sovereign Dialectic. Proposal: {}",
                    pattern, proposal
                )));
            }
        }

        let cwd = payload["cwd"].as_str();

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("powershell");
            c.args(["-Command", command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", command]);
            c
        };

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(SavantError::IoError)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let status = output.status.code().unwrap_or(-1);

        Ok(format!(
            "EXIT_CODE: {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
            status, stdout, stderr
        ))
    }

    fn capabilities(&self) -> savant_core::types::CapabilityGrants {
        savant_core::types::CapabilityGrants {
            ..Default::default()
        }
    }
}
