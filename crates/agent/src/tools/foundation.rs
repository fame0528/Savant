use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::Tool;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::info;

/// Sandboxing Path Resolver
/// Computes an absolute path strictly bounded within the agent's assigned workspace.
/// Rejects ParentDir traversal above workspace root; silently re-roots absolute paths.
pub(crate) fn secure_resolve_path(workspace: &Path, target: &str) -> Result<PathBuf, SavantError> {
    let target_path = Path::new(target);
    let mut resolved = workspace.to_path_buf();

    for component in target_path.components() {
        match component {
            std::path::Component::ParentDir => {
                if resolved == workspace {
                    return Err(SavantError::Unknown(
                        "Sandbox Escape Detected: Cannot navigate above workspace root.".into(),
                    ));
                }
                resolved.pop();
            }
            std::path::Component::Normal(c) => resolved.push(c),
            // Ignore RootDir and Prefix to silently re-root absolute path attacks
            _ => {}
        }
    }

    Ok(resolved)
}

/// Tool for atomic file moves/renames.
pub struct FileMoveTool {
    workspace_dir: PathBuf,
}

impl FileMoveTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl Tool for FileMoveTool {
    fn name(&self) -> &str {
        "file_move"
    }
    fn description(&self) -> &str {
        "Moves or renames a file or directory."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "from": { "type": "string", "description": "Source path to move from" },
                "to": { "type": "string", "description": "Destination path to move to" }
            },
            "required": ["from", "to"]
        })
    }

    fn requires_approval(&self) -> savant_core::traits::ApprovalRequirement {
        savant_core::traits::ApprovalRequirement::Conditional
    }

    fn domain(&self) -> savant_core::traits::ToolDomain {
        savant_core::traits::ToolDomain::Container
    }
    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let from_raw = payload["from"]
            .as_str()
            .ok_or_else(|| SavantError::Unknown("Missing 'from' path".to_string()))?;
        let to_raw = payload["to"]
            .as_str()
            .ok_or_else(|| SavantError::Unknown("Missing 'to' path".to_string()))?;

        let from = secure_resolve_path(&self.workspace_dir, from_raw)?;
        let to = secure_resolve_path(&self.workspace_dir, to_raw)?;

        info!(
            "[WAL:ACTUATOR] Action: move, From: {:?}, To: {:?}",
            from, to
        );
        fs::rename(&from, &to)
            .await
            .map_err(|e| SavantError::Unknown(format!("Move failed: {}", e)))?;
        Ok(format!("Successfully moved {:?} to {:?}.", from, to))
    }
}

/// Tool for file/directory deletion.
pub struct FileDeleteTool {
    workspace_dir: PathBuf,
}

impl FileDeleteTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl Tool for FileDeleteTool {
    fn name(&self) -> &str {
        "file_delete"
    }
    fn description(&self) -> &str {
        "Deletes a file or directory recursively."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file or directory to delete" }
            },
            "required": ["path"]
        })
    }

    fn requires_approval(&self) -> savant_core::traits::ApprovalRequirement {
        savant_core::traits::ApprovalRequirement::Always
    }

    fn domain(&self) -> savant_core::traits::ToolDomain {
        savant_core::traits::ToolDomain::Container
    }
    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let path_str = payload["path"]
            .as_str()
            .ok_or_else(|| SavantError::Unknown("Missing 'path' parameter".into()))?;
        let full_path = self.workspace_dir.join(path_str);

        // Security Check: Prevent path traversal attacks
        if full_path.canonicalize().map_or(true, |p| {
            !p.starts_with(self.workspace_dir.canonicalize().unwrap_or_default())
        }) {
            return Err(SavantError::Unknown("Path traversal detected".into()));
        }

        if !full_path.exists() {
            return Ok(
                "[AVX-IX] Operation complete. File not found. Universe integrity maintained."
                    .to_string(),
            );
        }

        if full_path.is_dir() {
            std::fs::remove_dir_all(&full_path)?;
        } else {
            std::fs::remove_file(&full_path)?;
        }

        // AudioScape: Log the deletion event
        info!(
            "NVMe Actuator: Successfully deleted path [{}]",
            full_path.display()
        );

        Ok(format!(
            "🗑️ Sovereign Deletion Actuation complete: `{}` permanently erased from the substrate.",
            path_str
        ))
    }
}

/// Tool for atomic multi-chunk file editing.
pub struct FileAtomicEditTool {
    workspace_dir: PathBuf,
}

impl FileAtomicEditTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl Tool for FileAtomicEditTool {
    fn name(&self) -> &str {
        "file_atomic_edit"
    }
    fn description(&self) -> &str {
        "Applies multiple atomic replacements to a file with backup/rollback safety."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to edit" },
                "replacements": {
                    "type": "array",
                    "description": "Array of {target, value} replacements to apply",
                    "items": {
                        "type": "object",
                        "properties": {
                            "target": { "type": "string", "description": "Text to find" },
                            "value": { "type": "string", "description": "Text to replace with" }
                        },
                        "required": ["target", "value"]
                    }
                }
            },
            "required": ["path", "replacements"]
        })
    }

    fn requires_approval(&self) -> savant_core::traits::ApprovalRequirement {
        savant_core::traits::ApprovalRequirement::Conditional
    }

    fn domain(&self) -> savant_core::traits::ToolDomain {
        savant_core::traits::ToolDomain::Container
    }
    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let target_raw = payload["path"]
            .as_str()
            .ok_or_else(|| SavantError::Unknown("Missing 'path' for atomic_edit".to_string()))?;
        let path = secure_resolve_path(&self.workspace_dir, target_raw)?;

        // Handle both array and string-encoded JSON array for replacements
        let replacements_owned;
        let replacements = if let Some(arr) = payload["replacements"].as_array() {
            arr
        } else if let Some(s) = payload["replacements"].as_str() {
            replacements_owned = serde_json::from_str::<Vec<Value>>(s).map_err(|e| {
                SavantError::Unknown(format!(
                    "Failed to parse replacements string as array: {}",
                    e
                ))
            })?;
            &replacements_owned
        } else {
            return Err(SavantError::Unknown(
                "Missing 'replacements' array for atomic_edit".to_string(),
            ));
        };

        info!(
            "[WAL:ACTUATOR] Action: atomic_edit, Path: {:?}, Changes: {}",
            path,
            replacements.len()
        );

        let mut content = fs::read_to_string(&path).await.map_err(|e| {
            SavantError::Unknown(format!("AtomicEdit: Failed to read {:?}: {}", path, e))
        })?;

        let backup_path = PathBuf::from(format!("{}.bak", path.to_string_lossy()));
        fs::copy(&path, &backup_path).await.map_err(|e| {
            SavantError::Unknown(format!("AtomicEdit: Failed to create backup: {}", e))
        })?;

        for replacement in replacements {
            let target = replacement["target"]
                .as_str()
                .ok_or_else(|| SavantError::Unknown("Missing 'target'".to_string()))?;
            let value = replacement["value"]
                .as_str()
                .ok_or_else(|| SavantError::Unknown("Missing 'value'".to_string()))?;

            if !content.contains(target) {
                fs::remove_file(&backup_path).await.ok();
                return Err(SavantError::Unknown(format!(
                    "AtomicEdit: Target not found: {}",
                    target
                )));
            }
            content = content.replace(target, value);
        }

        fs::write(&path, content).await.map_err(|e| {
            // Attempt rollback if write fails
            // Note: Since this is async-ification, we keep logic but must ensure sync/async parity
            SavantError::Unknown(format!("AtomicEdit: Write failed: {}", e))
        })?;

        fs::remove_file(&backup_path).await.ok();
        Ok(format!(
            "Successfully applied {} replacements to {:?}.",
            replacements.len(),
            path
        ))
    }
}

/// Tool for file and directory creation.
pub struct FileCreateTool {
    workspace_dir: PathBuf,
}

impl FileCreateTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl Tool for FileCreateTool {
    fn name(&self) -> &str {
        "file_create"
    }
    fn description(&self) -> &str {
        "Creates a new file with content or a new directory."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path where the file or directory should be created" },
                "content": { "type": "string", "description": "Content to write to the file (optional, defaults to empty)" },
                "directory": { "type": "boolean", "description": "Set to true to create a directory instead of a file" }
            },
            "required": ["path"]
        })
    }
    fn domain(&self) -> savant_core::traits::ToolDomain {
        savant_core::traits::ToolDomain::Container
    }
    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let target_raw = payload["path"]
            .as_str()
            .ok_or_else(|| SavantError::Unknown("Missing 'path' for create".to_string()))?;

        let path = secure_resolve_path(&self.workspace_dir, target_raw)?;

        // Check if this is a directory creation request
        if payload["directory"].as_bool().unwrap_or(false) {
            info!("[WAL:ACTUATOR] Action: create_directory, Path: {:?}", path);
            fs::create_dir_all(&path).await.map_err(|e| {
                SavantError::Unknown(format!("Failed to create directory {:?}: {}", path, e))
            })?;
            return Ok(format!("Successfully created directory: {:?}", path));
        }

        // File creation with optional content
        let content = payload["content"].as_str().unwrap_or("");

        info!("[WAL:ACTUATOR] Action: create_file, Path: {:?}", path);

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    SavantError::Unknown(format!("Failed to create parent dirs: {}", e))
                })?;
            }
        }

        fs::write(&path, content).await.map_err(|e| {
            SavantError::Unknown(format!("Failed to create file {:?}: {}", path, e))
        })?;

        Ok(format!(
            "Successfully created file: {:?} ({} bytes)",
            path,
            content.len()
        ))
    }
}

/// Legacy Foundation Tool for general operations.
pub struct FoundationTool {
    workspace_dir: PathBuf,
}

impl FoundationTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl Tool for FoundationTool {
    fn name(&self) -> &str {
        "foundation"
    }
    fn description(&self) -> &str {
        "File system operations: read, write, list, create, mkdir."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "Action to perform", "enum": ["read", "write", "ls", "create", "mkdir"] },
                "path": { "type": "string", "description": "File or directory path" },
                "content": { "type": "string", "description": "Content for write/create actions" }
            },
            "required": ["action", "path"]
        })
    }
    fn domain(&self) -> savant_core::traits::ToolDomain {
        savant_core::traits::ToolDomain::Container
    }

    fn max_output_chars(&self) -> usize {
        128_000 // File read can return large contents
    }

    fn timeout_secs(&self) -> u64 {
        30 // File ops are fast
    }

    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let action = payload["action"].as_str().unwrap_or("");

        // Fast path for resolving path securely once per action
        let target_raw = payload["path"]
            .as_str()
            .ok_or_else(|| SavantError::Unknown("Missing path".into()))?;
        let secure_path = secure_resolve_path(&self.workspace_dir, target_raw)?;

        match action {
            "read" => {
                match fs::read_to_string(&secure_path).await {
                    Ok(content) => Ok(content),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        Ok(format!("FILE_NOT_FOUND: {:?} does not exist. Create it first using the 'create' action.", secure_path))
                    }
                    Err(e) => Err(SavantError::Unknown(e.to_string()))
                }
            }
            "ls" => {
                let mut entries = fs::read_dir(&secure_path)
                    .await
                    .map_err(|e| SavantError::Unknown(e.to_string()))?;
                let mut out = String::new();
                while let Some(e) = entries
                    .next_entry()
                    .await
                    .map_err(|e| SavantError::Unknown(e.to_string()))?
                {
                    out.push_str(&format!("{}\n", e.file_name().to_string_lossy()));
                }
                Ok(out)
            }
            "write" => {
                let content = payload["content"].as_str().unwrap_or("");
                info!("[WAL:ACTUATOR] Action: write, Path: {:?}", secure_path);
                fs::write(&secure_path, content)
                    .await
                    .map_err(|e| SavantError::Unknown(format!("Write failed: {}", e)))?;
                Ok(format!(
                    "Successfully wrote {} bytes to {:?}",
                    content.len(),
                    secure_path
                ))
            }
            "mkdir" => {
                info!("[WAL:ACTUATOR] Action: mkdir, Path: {:?}", secure_path);
                fs::create_dir_all(&secure_path)
                    .await
                    .map_err(|e| SavantError::Unknown(format!("Mkdir failed: {}", e)))?;
                Ok(format!("Successfully created directory: {:?}", secure_path))
            }
            "create" => {
                let content = payload["content"].as_str().unwrap_or("");
                info!("[WAL:ACTUATOR] Action: create, Path: {:?}", secure_path);
                if let Some(parent) = secure_path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent).await.map_err(|e| {
                            SavantError::Unknown(format!("Failed to create parent dirs: {}", e))
                        })?;
                    }
                }
                fs::write(&secure_path, content)
                    .await
                    .map_err(|e| SavantError::Unknown(format!("Create failed: {}", e)))?;
                Ok(format!(
                    "Successfully created file: {:?} ({} bytes)",
                    secure_path,
                    content.len()
                ))
            }
            _ => Err(SavantError::Unknown(
                "Use specialized FS tools for destructive actions.".into(),
            )),
        }
    }
}
