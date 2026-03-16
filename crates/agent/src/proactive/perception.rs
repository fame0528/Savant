//! OMEGA-VIII: Perception Engine (Context Hydration)
//! 
//! Provides high-fidelity awareness of environment variance 
//! (Git changes, FS activity) to the proactive heartbeat.

use std::process::Command;
use std::path::Path;

pub struct PerceptionEngine;

impl PerceptionEngine {
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

    /// Checks for recent file system activity in the last 60 seconds.
    /// (Simplified: lists files modified in the last minute)
    pub fn get_fs_activity(path: &Path) -> String {
        // Note: On Windows, simple file list with timestamps can work
        // but for high-fidelity, we could use notify events if cached.
        // For now, we perform a quick glob for recently modified files.
        let mut activity = Vec::new();
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(elapsed) = modified.elapsed() {
                            if elapsed.as_secs() < 60 {
                                activity.push(format!("- {} (modified {}s ago)", 
                                    entry.file_name().to_string_lossy(),
                                    elapsed.as_secs()));
                            }
                        }
                    }
                }
            }
        }

        if activity.is_empty() {
            "No recent FS activity in immediate workspace.".to_string()
        } else {
            format!("Recent FS Activity:\n{}", activity.join("\n"))
        }
    }
}
