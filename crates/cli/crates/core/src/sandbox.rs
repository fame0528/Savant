use anyhow::{Context, Result};
use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, warn};

/// Safety level for a command
#[derive(Debug, Clone, PartialEq)]
pub enum CommandSafety {
    /// Safe to execute without approval
    Safe,
    /// Requires user approval
    RequiresApproval,
    /// Blocked entirely
    Blocked,
}

/// Result of a sandboxed command execution
#[derive(Debug, Clone)]
pub struct SandboxResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub compressed: bool,
}

/// Command execution sandbox with safety classification
pub struct Sandbox {
    safe_commands: Vec<String>,
    blocked_commands: Vec<String>,
    timeout_secs: u64,
    working_dir: Option<std::path::PathBuf>,
}

impl Sandbox {
    pub fn new() -> Self {
        Self {
            safe_commands: vec![
                "ls".to_string(),
                "cat".to_string(),
                "grep".to_string(),
                "rg".to_string(),
                "find".to_string(),
                "head".to_string(),
                "tail".to_string(),
                "wc".to_string(),
                "echo".to_string(),
                "pwd".to_string(),
                "git".to_string(),
                "cargo".to_string(),
                "npm".to_string(),
                "yarn".to_string(),
                "node".to_string(),
                "python".to_string(),
                "python3".to_string(),
                "rustc".to_string(),
                "cargo".to_string(),
                "cargo check".to_string(),
                "cargo test".to_string(),
                "cargo fmt".to_string(),
                "cargo clippy".to_string(),
                "npm test".to_string(),
                "npm run test".to_string(),
                "git status".to_string(),
                "git diff".to_string(),
                "git log".to_string(),
                "git branch".to_string(),
            ],
            blocked_commands: vec![
                "rm -rf /".to_string(),
                "mkfs".to_string(),
                "dd".to_string(),
                "curl".to_string(),
                "wget".to_string(),
                "nc".to_string(),
                "ncat".to_string(),
                "socat".to_string(),
                "ssh".to_string(),
                "scp".to_string(),
                "eval".to_string(),
                "source".to_string(),
                "exec".to_string(),
                "chmod".to_string(),
                "chown".to_string(),
                "sudo".to_string(),
                "su".to_string(),
                "shutdown".to_string(),
                "reboot".to_string(),
                "halt".to_string(),
                "poweroff".to_string(),
                "init".to_string(),
                "kill".to_string(),
                "killall".to_string(),
                "pkill".to_string(),
            ],
            timeout_secs: 30,
            working_dir: None,
        }
    }

    /// Set working directory for command execution
    pub fn with_working_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }

    /// Set execution timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Classify a command's safety level
    pub fn classify(&self, cmd: &str) -> CommandSafety {
        let cmd_lower = cmd.to_lowercase().trim().to_string();

        // Check blocked list first
        for blocked in &self.blocked_commands {
            if cmd_lower.starts_with(blocked) {
                return CommandSafety::Blocked;
            }
        }

        // Check safe list
        for safe in &self.safe_commands {
            if cmd_lower.starts_with(safe) {
                return CommandSafety::Safe;
            }
        }

        // Destructive commands require approval
        if cmd_lower.starts_with("rm ")
            || cmd_lower.starts_with("del ")
            || cmd_lower.starts_with("git push")
            || cmd_lower.starts_with("git reset --hard")
            || cmd_lower.starts_with("npm publish")
            || cmd_lower.starts_with("cargo publish")
            || cmd_lower.starts_with("docker")
        {
            return CommandSafety::RequiresApproval;
        }

        // Unknown commands require approval
        CommandSafety::RequiresApproval
    }

    /// Execute a command in the sandbox
    pub async fn execute(&self, cmd: &str) -> Result<SandboxResult> {
        let safety = self.classify(cmd);
        match safety {
            CommandSafety::Blocked => {
                return Err(anyhow::anyhow!("Command blocked by sandbox: {}", cmd));
            }
            CommandSafety::RequiresApproval => {
                warn!("Command requires approval: {}", cmd);
            }
            CommandSafety::Safe => {
                debug!("Executing safe command: {}", cmd);
            }
        }

        let start = std::time::Instant::now();

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command"));
        }

        let mut command = Command::new(parts[0]);
        for arg in &parts[1..] {
            command.arg(arg);
        }

        if let Some(ref dir) = self.working_dir {
            command.current_dir(dir);
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let output: std::process::Output = timeout(
            Duration::from_secs(self.timeout_secs),
            tokio::task::spawn_blocking(move || command.output()),
        )
        .await
        .with_context(|| format!("Command timed out after {}s: {}", self.timeout_secs, cmd))?
        .map_err(|e| anyhow::anyhow!("Join error: {}", e))?
        .with_context(|| format!("Command failed: {}", cmd))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        debug!(
            "Command exited with code {} in {}ms",
            exit_code, duration_ms
        );

        Ok(SandboxResult {
            stdout,
            stderr,
            exit_code,
            duration_ms,
            compressed: false,
        })
    }

    /// Get list of safe commands
    pub fn safe_commands(&self) -> &[String] {
        &self.safe_commands
    }

    /// Get list of blocked commands
    pub fn blocked_commands(&self) -> &[String] {
        &self.blocked_commands
    }
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::new()
    }
}
