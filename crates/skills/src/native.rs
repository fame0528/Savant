//
use savant_core::error::SavantError;
use async_trait::async_trait;

/// A trusted native skill implemented statically in rust.
pub struct FileSystemSkill;

impl FileSystemSkill {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileSystemSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl savant_core::traits::Tool for FileSystemSkill {
    fn name(&self) -> &str { "filesystem" }
    fn description(&self) -> &str { "Trusted filesystem operations." }
    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError> {
        let action = payload.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SavantError::Unknown("Missing 'action' field".into()))?;

        match action {
            "read" => {
                let path = payload.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SavantError::Unknown("Missing 'path' field".into()))?;
                let content = std::fs::read_to_string(path)
                    .map_err(|e| SavantError::Unknown(format!("Read failed: {}", e)))?;
                Ok(content)
            }
            "write" => {
                let path = payload.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SavantError::Unknown("Missing 'path' field".into()))?;
                let content = payload.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SavantError::Unknown("Missing 'content' field".into()))?;
                std::fs::write(path, content)
                    .map_err(|e| SavantError::Unknown(format!("Write failed: {}", e)))?;
                Ok("Success".into())
            }
            _ => Err(SavantError::Unknown(format!("Unknown action: {}", action))),
        }
    }
}
