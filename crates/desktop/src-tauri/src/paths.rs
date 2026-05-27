use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Centralized path resolver.
///
/// Resolves all Savant data paths relative to the Tauri application's
/// data directory, with fallback to the project root for development builds.
pub struct SavantPathResolver {
    pub base_data_path: PathBuf,
    pub base_config_path: PathBuf,
}

impl SavantPathResolver {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        // Use Tauri's app data directory for production builds
        // Falls back to the project root if the app data directory is unavailable
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to resolve app data directory: {}", e))?;

        // Ensure the data directory exists
        if !app_data_dir.exists() {
            std::fs::create_dir_all(&app_data_dir).map_err(|e| {
                format!(
                    "Failed to create app data directory at {:?}: {}",
                    app_data_dir, e
                )
            })?;
        }

        // Config lives alongside data in a config subdirectory
        let config_dir = app_data_dir.join("config");
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).map_err(|e| {
                format!(
                    "Failed to create config directory at {:?}: {}",
                    config_dir, e
                )
            })?;
        }

        Ok(Self {
            base_data_path: app_data_dir,
            base_config_path: config_dir,
        })
    }

    pub fn config_file(&self) -> PathBuf {
        self.base_config_path.join("config").join("savant.toml")
    }

    pub fn env_file(&self) -> PathBuf {
        self.base_data_path.join(".env")
    }

    pub fn workspaces_dir(&self) -> PathBuf {
        self.base_data_path.join("workspaces")
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.base_data_path.join("skills")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.base_data_path.join("data").join("savant")
    }

    pub fn memory_dir(&self) -> PathBuf {
        self.base_data_path.join("data").join("memory")
    }
}
