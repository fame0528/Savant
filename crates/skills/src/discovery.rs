use std::path::{Path, PathBuf};
use savant_core::error::SavantError;
use savant_core::traits::SkillExecutor;
use std::sync::Arc;
use crate::wasm::WasmSkillExecutor;

/// Represents a single available skill for an agent.
pub struct Skill {
    pub name: String,
    pub description: String,
    pub source: PathBuf,
    pub permissions: Vec<String>,
    pub setup_schema: Option<serde_json::Value>, // Feature 5: Managed Skill Wizards
    pub executor: Arc<dyn SkillExecutor>,
}

/// Scans a directory structure for SKILL.md and available skill models.
/// Returns a collection of fully initialized Skills.
pub fn discover_skills(base_path: &Path) -> Result<Vec<Skill>, SavantError> {
    let mut skills = Vec::new();
    
    if !base_path.exists() {
        return Ok(skills);
    }

    for entry in walkdir::WalkDir::new(base_path) {
        let entry = entry.map_err(|e| SavantError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        if entry.file_name() == "SKILL.md" {
            let skill_dir = entry.path().parent().unwrap();
            if let Ok(skill) = load_skill(skill_dir) {
                skills.push(skill);
            }
        }
    }

    Ok(skills)
}

fn load_skill(path: &Path) -> Result<Skill, SavantError> {
    let manifest_path = path.join("SKILL.md");
    let content = std::fs::read_to_string(manifest_path)?;
    
    // Simple line-based metadata extraction for now
    let mut name = "Unknown Skill".to_string();
    let mut description = String::new();
    
    for line in content.lines() {
        if let Some(n) = line.strip_prefix("# ") {
            name = n.trim().to_string();
        } else if !line.starts_with('#') && !line.trim().is_empty() && description.is_empty() {
            description = line.trim().to_string();
        }
    }

    // Look for .wasm file
    let wasm_file = std::fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map_or(false, |ext| ext == "wasm"))
        .ok_or_else(|| SavantError::Unknown(format!("No .wasm file found in skill dir: {}", path.display())))?;

    let wasm_bytes = std::fs::read(wasm_file.path())?;
    let executor = Arc::new(WasmSkillExecutor::new(&wasm_bytes)?);

    Ok(Skill {
        name,
        description,
        source: path.to_path_buf(),
        permissions: Vec::new(),
        setup_schema: None,
        executor,
    })
}

#[cfg(test)]
mod benches {
    // criterion benchmark stub
}
