use std::path::Path;
use tokio::fs;
use crate::error::SavantError;


/// Reads a mandatory file or returns a default string.
pub async fn read_or_default(path: &Path, default: &str) -> String {
    if path.exists() {
        fs::read_to_string(path).await.unwrap_or_else(|_| default.to_string())
    } else {
        default.to_string()
    }
}

/// Reads an optional file returning None if missing.
pub async fn read_optional(path: &Path) -> Option<String> {
    if path.exists() {
        fs::read_to_string(path).await.ok()
    } else {
        None
    }
}

/// Appends a line to a .env file in the specified directory.
pub async fn append_to_env(workspace_path: &Path, key: &str, value: &str) -> Result<(), SavantError> {
    let env_path = workspace_path.join(".env");
    let line = format!("{}={}\n", key, value);
    
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(env_path)
        .await?;
        
    use tokio::io::AsyncWriteExt;
    file.write_all(line.as_bytes()).await?;
    Ok(())
}

/// Ensures a directory exists.
pub async fn ensure_dir(path: &Path) -> Result<(), SavantError> {
    if !path.exists() {
        fs::create_dir_all(path).await?;
    }
    Ok(())
}
