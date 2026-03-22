//! OMEGA-VIII: Task Matrix (Autonomous Orchestration)
//!
//! Manages persistent, externalized work queues for proactive agents.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskItem {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Copy)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

pub struct TaskMatrix {
    path: PathBuf,
}

impl TaskMatrix {
    pub fn new(root: &Path, config: &savant_core::config::ProactiveConfig) -> Self {
        Self {
            path: root.join(&config.task_matrix_file),
        }
    }

    /// Loads the current task matrix from the markdown file.
    pub fn load_tasks(&self) -> Vec<TaskItem> {
        if !self.path.exists() {
            return Vec::new();
        }

        let content = fs::read_to_string(&self.path).unwrap_or_default();
        let mut tasks = Vec::new();

        for line in content.lines() {
            if line.starts_with("- [ ]") || line.starts_with("- [/]") || line.starts_with("- [x]") {
                let status = if line.contains("[ ]") {
                    TaskStatus::Pending
                } else if line.contains("[/]") {
                    TaskStatus::InProgress
                } else {
                    TaskStatus::Completed
                };

                let desc = line[6..].trim().to_string();
                tasks.push(TaskItem {
                    id: uuid::Uuid::new_v4().to_string(), // Transient ID for session
                    description: desc,
                    status,
                    priority: 1,
                });
            }
        }
        tasks
    }

    /// Appends a new task to the matrix.
    pub fn add_task(&self, task: &str) -> std::io::Result<()> {
        let line = format!("\n- [ ] {}", task);
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        use std::io::Write;
        file.write_all(line.as_bytes())
    }

    /// Returns a formatted string of pending tasks for prompt injection.
    pub fn get_pending_summary(&self) -> String {
        let tasks = self.load_tasks();
        let pending: Vec<String> = tasks
            .into_iter()
            .filter(|t| t.status == TaskStatus::Pending || t.status == TaskStatus::InProgress)
            .map(|t| format!("- {}", t.description))
            .collect();

        if pending.is_empty() {
            "No pending orchestration tasks.".to_string()
        } else {
            format!("PENDING TASKS (Task Matrix):\n{}", pending.join("\n"))
        }
    }

    /// OMEGA-VIII: Toggle task status in the markdown file.
    pub fn toggle_task(&self, description: &str, status: TaskStatus) -> std::io::Result<()> {
        let content = fs::read_to_string(&self.path)?;
        let mut new_lines = Vec::new();
        let target_prefix = match status {
            TaskStatus::Pending => "- [ ]",
            TaskStatus::InProgress => "- [/]",
            TaskStatus::Completed => "- [x]",
            TaskStatus::Failed => "- [!] ",
        };

        for line in content.lines() {
            let trimmed = line.trim();
            // Check for exact task match with boundary
            let matches_task = if trimmed.starts_with("- [") && trimmed.len() > 6 {
                let current_desc = &trimmed[6..].trim();
                *current_desc == description
            } else {
                false
            };

            if matches_task {
                new_lines.push(format!("{} {}", target_prefix, description));
            } else {
                new_lines.push(line.to_string());
            }
        }

        fs::write(&self.path, new_lines.join("\n"))?;
        Ok(())
    }
}
