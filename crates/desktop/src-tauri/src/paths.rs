use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Centralized path resolver.
/// For now: hardcoded to Spencer's dev project. Will be generalized for public release.
pub struct SavantPathResolver {
    pub base_data_path: PathBuf,
    pub base_config_path: PathBuf,
}

impl SavantPathResolver {
    pub fn new(_app: &AppHandle) -> Result<Self, String> {
        // Spencer's dev project — hardcoded for now
        let dev_root = PathBuf::from("C:\\Users\\spenc\\dev\\Savant");
        if dev_root.exists() {
            Ok(Self {
                base_data_path: dev_root.clone(),
                base_config_path: dev_root,
            })
        } else {
            Err("Dev project not found at C:\\Users\\spenc\\dev\\Savant".to_string())
        }
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
