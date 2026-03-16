use savant_core::traits::Tool;
use savant_core::error::SavantError;
use crate::orchestration::tasks::{TaskMatrix, TaskStatus};
use async_trait::async_trait;
use std::path::PathBuf;

/// OMEGA-VIII: Task Matrix Management Tool
/// Allows agents to autonomously update their orchestration state.
pub struct TaskMatrixTool {
    workspace_path: PathBuf,
    config: savant_core::config::ProactiveConfig,
}

impl TaskMatrixTool {
    pub fn new(workspace_path: PathBuf, config: savant_core::config::ProactiveConfig) -> Self {
        Self { workspace_path, config }
    }
}

#[async_trait]
impl Tool for TaskMatrixTool {
    fn name(&self) -> &str {
        "update_task_status"
    }

    fn description(&self) -> &str {
        "Updates the status of a task in the orchestration matrix. \
        Parameters (as JSON): { \"description\": \"task text\", \"status\": \"Pending|InProgress|Completed|Failed\" }"
    }

    fn capabilities(&self) -> savant_core::types::CapabilityGrants {
        savant_core::types::CapabilityGrants::default()
    }

    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError> {
        let matrix = TaskMatrix::new(&self.workspace_path, &self.config);
        
        let description = payload["description"]
            .as_str()
            .ok_or_else(|| SavantError::Unknown("Missing 'description' parameter".to_string()))?;
            
        let status_str = payload["status"]
            .as_str()
            .ok_or_else(|| SavantError::Unknown("Missing 'status' parameter".to_string()))?
            .to_lowercase();
        
        let status = match status_str.as_str() {
            "pending" => TaskStatus::Pending,
            "inprogress" | "in_progress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            _ => return Ok(format!("Error: Invalid status '{}'", status_str)),
        };

        match matrix.toggle_task(description, status) {
            Ok(_) => Ok(format!("Successfully updated task '{}' to {:?}", description, status)),
            Err(e) => Ok(format!("Error updating task: {}", e)),
        }
    }
}
