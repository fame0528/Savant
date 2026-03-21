//! OMEGA-VIII: Perception Engine (Context Hydration)
//!
//! Provides high-fidelity awareness of environment variance
//! (Git changes, FS activity) to the proactive heartbeat.

use std::path::Path;
use std::process::Command;

/// Perception configuration with tunable thresholds.
pub struct PerceptionConfig {
    /// How many seconds back to check for file modifications.
    pub fs_activity_window_secs: u64,
    /// How many files to list at most in activity report.
    pub max_activity_entries: usize,
}

impl Default for PerceptionConfig {
    fn default() -> Self {
        Self {
            fs_activity_window_secs: 60,
            max_activity_entries: 20,
        }
    }
}

/// Perception engine with configurable thresholds.
pub struct PerceptionEngine {
    config: PerceptionConfig,
}

impl PerceptionEngine {
    pub fn new(config: PerceptionConfig) -> Self {
        Self { config }
    }

    pub fn default_engine() -> Self {
        Self::new(PerceptionConfig::default())
    }

    /// Captures a high-level summary of Git changes in the workspace.
    pub fn get_git_status(path: &Path) -> String {
        let output = Command::new("git")
            .args(["status", "--short"])
            .current_dir(path)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let s = String::from_utf8_lossy(&out.stdout).to_string();
                if s.is_empty() {
                    "No pending git changes.".to_string()
                } else {
                    format!("Git Status:\n{}", s)
                }
            }
            _ => "Git status unavailable.".to_string(),
        }
    }

    /// Captures a brief diff of the most recent changes.
    pub fn get_git_diff(path: &Path) -> String {
        let output = Command::new("git")
            .args(["diff", "--stat"])
            .current_dir(path)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let s = String::from_utf8_lossy(&out.stdout).to_string();
                if s.is_empty() {
                    "".to_string()
                } else {
                    format!("Git Diff Summary:\n{}", s)
                }
            }
            _ => "".to_string(),
        }
    }

    /// Checks for recent file system activity within the configured time window.
    pub fn get_fs_activity(&self, path: &Path) -> String {
        let window_secs = self.config.fs_activity_window_secs;
        let max_entries = self.config.max_activity_entries;
        let mut activity = Vec::new();

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if activity.len() >= max_entries {
                    break;
                }
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(elapsed) = modified.elapsed() {
                            if elapsed.as_secs() < window_secs {
                                activity.push(format!(
                                    "- {} (modified {}s ago)",
                                    entry.file_name().to_string_lossy(),
                                    elapsed.as_secs()
                                ));
                            }
                        }
                    }
                }
            }
        }

        if activity.is_empty() {
            format!("No recent FS activity in last {}s.", window_secs)
        } else {
            format!(
                "Recent FS Activity (last {}s):\n{}",
                window_secs,
                activity.join("\n")
            )
        }
    }

    /// OMEGA-VIII: High-Fidelity Substrate Metrics (Phase 5)
    pub fn get_substrate_metrics() -> String {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", "Get-CimInstance Win32_OperatingSystem | Select-Object FreePhysicalMemory,TotalVisibleMemorySize | ConvertTo-Json"])
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    let s = String::from_utf8_lossy(&out.stdout);
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                        let free = v["FreePhysicalMemory"].as_u64().unwrap_or(0) / 1024; // KB to MB
                        let total = v["TotalVisibleMemorySize"].as_u64().unwrap_or(0) / 1024; // KB to MB
                        let used = total.saturating_sub(free);
                        let usage_pct = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };
                        return format!("Substrate Metrics (OS):\n- Memory: {}MB / {}MB ({:.1}%)", used, total, usage_pct);
                    }
                }
                _ => {}
            }
        }

        "Substrate metrics (OS) unavailable.".to_string()
    }
}
