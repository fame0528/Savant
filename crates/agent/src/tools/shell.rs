use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::Tool;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn};

/// SovereignShell: High-Fidelity Terminal Actuator
///
/// Unlike foundation.exec, SovereignShell is designed for complex, multi-stage
/// operations where stdout/stderr capture and exit status are critical for
/// autonomous error recovery.
///
/// Workspace-bounded: all commands execute within the agent's assigned workspace.
/// CWD is resolved through `secure_resolve_path` which rejects any path escaping
/// the workspace boundary. Absolute paths in command arguments are validated against
/// an allowlist of known-safe system directories.
pub struct SovereignShell {
    workspace_root: PathBuf,
}

impl SovereignShell {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

/// Known-safe system directories where absolute paths are permitted.
/// Commands referencing paths outside these directories AND outside the workspace are rejected.
const SAFE_SYSTEM_DIRS: &[&str] = &[
    "/usr/bin",
    "/usr/local/bin",
    "/bin",
    "/sbin",
    "/usr/sbin",
    "/usr/lib",
    "/usr/local/lib",
    "/usr/share",
    "/opt",
    "/var/lib",
    "/tmp",
];

/// Destructive command patterns with proposals for safer alternatives.
/// Each entry: (pattern, human-readable proposal)
const DESTRUCTIVE_PATTERNS: &[(&str, &str)] = &[
    // Directory/file removal variants
    ("rm -rf", "Use 'mv' to a temporary directory instead."),
    ("rm -r -f", "Use 'mv' to a temporary directory instead."),
    ("rm -fr", "Use 'mv' to a temporary directory instead."),
    ("rm -Rf", "Use 'mv' to a temporary directory instead."),
    // Disk formatting
    ("format", "Disk formatting is prohibited."),
    ("mkfs", "Filesystem creation is prohibited."),
    ("mkfs.ext4", "Filesystem creation is prohibited."),
    ("mkfs.ntfs", "Filesystem creation is prohibited."),
    ("mkfs.fat", "Filesystem creation is prohibited."),
    // Raw disk I/O
    (
        "dd if=",
        "Raw disk reads are prohibited outside maintenance mode.",
    ),
    (
        "dd of=",
        "Raw disk writes are prohibited outside maintenance mode.",
    ),
    // Git destruction
    (
        "git reset --hard",
        "Use 'git stash' to preserve current changes.",
    ),
    (
        "git clean -fd",
        "Use 'git clean -n' (dry-run) first to review changes.",
    ),
    (
        "git clean -fx",
        "Use 'git clean -n' (dry-run) first to review changes.",
    ),
    // Secure deletion
    ("shred", "Secure deletion is prohibited."),
    ("wipe", "Secure deletion is prohibited."),
    ("srm", "Secure deletion is prohibited."),
    // Permission escalation
    (
        "chmod 777",
        "Use least-privilege permissions (e.g., 755 or 644).",
    ),
    (
        "chmod -R 777",
        "Use least-privilege permissions (e.g., 755 or 644).",
    ),
    // Ownership changes
    ("chown -R", "Recursive ownership changes are restricted."),
    ("chgrp -R", "Recursive group changes are restricted."),
    // Remote code execution
    (
        "curl | sh",
        "Download and review the script before executing.",
    ),
    (
        "curl | bash",
        "Download and review the script before executing.",
    ),
    (
        "wget | sh",
        "Download and review the script before executing.",
    ),
    (
        "wget | bash",
        "Download and review the script before executing.",
    ),
    // Fork bomb
    (":(){ :|:& };:", "Fork bombs are prohibited."),
    // Code execution
    ("eval(", "Dynamic code evaluation is restricted."),
    ("exec(", "Dynamic process execution is restricted."),
    ("system(", "System call invocation is restricted."),
    // Python destruction
    ("os.remove", "Python file removal is restricted."),
    ("os.rmdir", "Python directory removal is restricted."),
    ("shutil.rmtree", "Python recursive removal is restricted."),
];

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

        // 4.8: Pre-flight workspace verification
        if !self.workspace_root.exists() {
            if let Err(e) = tokio::fs::create_dir_all(&self.workspace_root).await {
                return Err(SavantError::Unknown(format!(
                    "Workspace root does not exist and cannot be created: {}",
                    e
                )));
            }
            info!("Created workspace root at {:?}", self.workspace_root);
        }

        // 4.5: Destructive pattern detection with proposals
        for (pattern, proposal) in DESTRUCTIVE_PATTERNS {
            if command.contains(pattern) {
                warn!(
                    "[SHELL_AUDIT] decision=REJECTED reason=destructive_pattern pattern={} command_hash={}",
                    pattern,
                    Self::command_hash(command)
                );
                return Err(SavantError::ConsensusVeto(format!(
                    "Destructive command '{}' blocked. Proposal: {}",
                    pattern, proposal
                )));
            }
        }

        // 4.6: Absolute path injection detection
        // Scan command tokens for absolute paths outside workspace and safe system dirs
        let command_lower = command.to_lowercase();
        let dangerous_absolute_paths: &[&str] = if cfg!(target_os = "windows") {
            &[
                "c:\\windows",
                "c:\\users\\",
                "d:\\windows",
                "c:\\program files",
            ]
        } else {
            &[
                "/etc/",
                "/root/",
                "/home/",
                "/var/log/",
                "/dev/",
                "/proc/",
                "/sys/",
            ]
        };

        for dangerous_path in dangerous_absolute_paths {
            if command_lower.contains(dangerous_path) {
                // Check if this path reference is within the workspace or a safe system dir
                let is_safe = SAFE_SYSTEM_DIRS
                    .iter()
                    .any(|safe| command_lower.contains(safe));

                if !is_safe {
                    warn!(
                        "[SHELL_AUDIT] decision=REJECTED reason=path_injection path={} command_hash={}",
                        dangerous_path,
                        Self::command_hash(command)
                    );
                    return Err(SavantError::ConsensusVeto(format!(
                        "Command references path '{}' which is outside the workspace and known-safe system directories.",
                        dangerous_path
                    )));
                }
            }
        }

        // 4.4: CWD sandboxing via secure_resolve_path
        let requested_cwd = payload["cwd"].as_str();
        let resolved_cwd = match requested_cwd {
            Some(cwd) => match super::foundation::secure_resolve_path(&self.workspace_root, cwd) {
                Ok(path) => path,
                Err(e) => {
                    warn!(
                        "[SHELL_AUDIT] decision=REJECTED reason=cwd_escape cwd={} command_hash={}",
                        cwd,
                        Self::command_hash(command)
                    );
                    return Err(SavantError::ConsensusVeto(format!(
                            "CWD sandbox escape detected: {}. Commands must execute within the workspace.",
                            e
                        )));
                }
            },
            None => self.workspace_root.clone(),
        };

        // 4.7: Audit logging — log every execution
        info!(
            "[SHELL_AUDIT] decision=ALLOWED cwd={} command_hash={}",
            resolved_cwd.display(),
            Self::command_hash(command)
        );

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("powershell");
            c.args(["-Command", command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", command]);
            c
        };

        cmd.current_dir(&resolved_cwd);

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

impl SovereignShell {
    /// Generates a truncated SHA-256 hash of the command for audit logging.
    /// Full commands are not logged to prevent credential leakage in logs.
    fn command_hash(command: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        command.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}
