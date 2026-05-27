//! Workspace module — project/workspace state management

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub root: Option<String>,
    pub name: Option<String>,
}

impl WorkspaceState {
    pub fn set_root(&mut self, path: String) {
        self.name = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());
        self.root = Some(path);
    }
}
