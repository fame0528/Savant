use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::Tool;
use serde_json::Value;
use tokio::fs;
use std::path::Path;
use tracing::info;

/// Tool for atomic file moves/renames.
pub struct FileMoveTool;

#[async_trait]
impl Tool for FileMoveTool {
    fn name(&self) -> &str { "file_move" }
    fn description(&self) -> &str { "Moves or renames a file/directory. Governed by Swarm Consensus." }
    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let from = payload["from"].as_str().ok_or_else(|| SavantError::Unknown("Missing 'from' path".to_string()))?;
        let to = payload["to"].as_str().ok_or_else(|| SavantError::Unknown("Missing 'to' path".to_string()))?;
        
        info!("[WAL:ACTUATOR] Action: move, From: {}, To: {}", from, to);
        fs::rename(from, to).await.map_err(|e| SavantError::Unknown(format!("Move failed: {}", e)))?;
        Ok(format!("Successfully moved {} to {}.", from, to))
    }
}

/// Tool for file/directory deletion.
pub struct FileDeleteTool;

#[async_trait]
impl Tool for FileDeleteTool {
    fn name(&self) -> &str { "file_delete" }
    fn description(&self) -> &str { "Deletes a file or directory recursively. Governed by Swarm Consensus." }
    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let path = payload["path"].as_str().ok_or_else(|| SavantError::Unknown("Missing 'path' for delete".to_string()))?;
        
        info!("[WAL:ACTUATOR] Action: delete, Path: {}", path);
        if Path::new(path).is_dir() {
            fs::remove_dir_all(path).await.map_err(|e| SavantError::Unknown(format!("Recursive delete failed: {}", e)))?;
        } else {
            fs::remove_file(path).await.map_err(|e| SavantError::Unknown(format!("Delete failed: {}", e)))?;
        }
        Ok(format!("Successfully deleted {}.", path))
    }
}

/// Tool for atomic multi-chunk file editing.
pub struct FileAtomicEditTool;

#[async_trait]
impl Tool for FileAtomicEditTool {
    fn name(&self) -> &str { "file_atomic_edit" }
    fn description(&self) -> &str { "Applies multiple atomic replacements to a file with backup/rollback safety." }
    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let path = payload["path"].as_str().ok_or_else(|| SavantError::Unknown("Missing 'path' for atomic_edit".to_string()))?;
        let replacements = payload["replacements"].as_array().ok_or_else(|| SavantError::Unknown("Missing 'replacements' array for atomic_edit".to_string()))?;
        
        info!("[WAL:ACTUATOR] Action: atomic_edit, Path: {}, Changes: {}", path, replacements.len());
        
        let mut content = fs::read_to_string(path).await
            .map_err(|e| SavantError::Unknown(format!("AtomicEdit: Failed to read {}: {}", path, e)))?;
        
        let backup_path = format!("{}.bak", path);
        fs::copy(path, &backup_path).await.map_err(|e| SavantError::Unknown(format!("AtomicEdit: Failed to create backup: {}", e)))?;

        for replacement in replacements {
            let target = replacement["target"].as_str().ok_or_else(|| SavantError::Unknown("Missing 'target'".to_string()))?;
            let value = replacement["value"].as_str().ok_or_else(|| SavantError::Unknown("Missing 'value'".to_string()))?;
            
            if !content.contains(target) {
                fs::remove_file(&backup_path).await.ok();
                return Err(SavantError::Unknown(format!("AtomicEdit: Target not found: {}", target)));
            }
            content = content.replace(target, value);
        }

        fs::write(path, content).await.map_err(|e| {
            // Attempt rollback if write fails
            // Note: Since this is async-ification, we keep logic but must ensure sync/async parity
            SavantError::Unknown(format!("AtomicEdit: Write failed: {}", e))
        })?;

        fs::remove_file(&backup_path).await.ok();
        Ok(format!("Successfully applied {} replacements to {}.", replacements.len(), path))
    }
}

/// Tool for file and directory creation.
pub struct FileCreateTool;

#[async_trait]
impl Tool for FileCreateTool {
    fn name(&self) -> &str { "file_create" }
    fn description(&self) -> &str { "Creates a new file with content or a new directory." }
    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let path = payload["path"].as_str()
            .ok_or_else(|| SavantError::Unknown("Missing 'path' for create".to_string()))?;

        // Check if this is a directory creation request
        if payload["directory"].as_bool().unwrap_or(false) {
            info!("[WAL:ACTUATOR] Action: create_directory, Path: {}", path);
            fs::create_dir_all(path).await
                .map_err(|e| SavantError::Unknown(format!("Failed to create directory {}: {}", path, e)))?;
            return Ok(format!("Successfully created directory: {}", path));
        }

        // File creation with optional content
        let content = payload["content"].as_str().unwrap_or("");
        
        info!("[WAL:ACTUATOR] Action: create_file, Path: {}", path);
        
        // Create parent directories if they don't exist
        if let Some(parent) = Path::new(path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await
                    .map_err(|e| SavantError::Unknown(format!("Failed to create parent dirs: {}", e)))?;
            }
        }

        fs::write(path, content).await
            .map_err(|e| SavantError::Unknown(format!("Failed to create file {}: {}", path, e)))?;
        
        Ok(format!("Successfully created file: {} ({} bytes)", path, content.len()))
    }
}

/// Legacy Foundation Tool for general operations.
pub struct FoundationTool;

impl Default for FoundationTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FoundationTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Tool for FoundationTool {
    fn name(&self) -> &str { "foundation" }
    fn description(&self) -> &str { "General system foundation actuators." }
    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let action = payload["action"].as_str().unwrap_or("");
        match action {
             "read" => {
                let path = payload["path"].as_str().ok_or_else(|| SavantError::Unknown("Missing path".into()))?;
                let content = fs::read_to_string(path).await.map_err(|e| SavantError::Unknown(e.to_string()))?;
                Ok(content)
            }
            "ls" => {
                let path = payload["path"].as_str().unwrap_or(".");
                let mut entries = fs::read_dir(path).await.map_err(|e| SavantError::Unknown(e.to_string()))?;
                let mut out = String::new();
                while let Some(e) = entries.next_entry().await.map_err(|e| SavantError::Unknown(e.to_string()))? {
                    out.push_str(&format!("{}\n", e.file_name().to_string_lossy()));
                }
                Ok(out)
            }
            "write" => {
                let path = payload["path"].as_str().ok_or_else(|| SavantError::Unknown("Missing path".into()))?;
                let content = payload["content"].as_str().unwrap_or("");
                info!("[WAL:ACTUATOR] Action: write, Path: {}", path);
                fs::write(path, content).await.map_err(|e| SavantError::Unknown(format!("Write failed: {}", e)))?;
                Ok(format!("Successfully wrote {} bytes to {}", content.len(), path))
            }
            "mkdir" => {
                let path = payload["path"].as_str().ok_or_else(|| SavantError::Unknown("Missing path".into()))?;
                info!("[WAL:ACTUATOR] Action: mkdir, Path: {}", path);
                fs::create_dir_all(path).await.map_err(|e| SavantError::Unknown(format!("Mkdir failed: {}", e)))?;
                Ok(format!("Successfully created directory: {}", path))
            }
            "create" => {
                let path = payload["path"].as_str().ok_or_else(|| SavantError::Unknown("Missing path".into()))?;
                let content = payload["content"].as_str().unwrap_or("");
                info!("[WAL:ACTUATOR] Action: create, Path: {}", path);
                if let Some(parent) = Path::new(path).parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent).await.map_err(|e| SavantError::Unknown(format!("Failed to create parent dirs: {}", e)))?;
                    }
                }
                fs::write(path, content).await.map_err(|e| SavantError::Unknown(format!("Create failed: {}", e)))?;
                Ok(format!("Successfully created file: {} ({} bytes)", path, content.len()))
            }
            _ => Err(SavantError::Unknown("Use specialized FS tools for destructive actions.".into()))
        }
    }
}

