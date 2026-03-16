use std::fs;
use std::path::PathBuf;
use tracing::info;

/// Protocol S-ATLAS: Autonomous Lesson Distillation (ALD)
/// Distills raw learnings into core sovereign SOUL files.
pub struct ALDEngine {
    workspace_root: PathBuf,
}

impl ALDEngine {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Run a distillation cycle.
    pub fn distill(&self) -> Result<(), Box<dyn std::error::Error>> {
        let learnings_path = self.workspace_root.join("LEARNINGS.md");
        if !learnings_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&learnings_path)?;
        let blocks: Vec<&str> = content.split("### Learning").collect();
        
        // AAA: Heuristic - Only promote deep reflections (>500 chars or specific keywords)
        for block in blocks.iter().skip(1) {
            if block.contains("Strategic Insight") || block.contains("Empire Impact") {
                self.promote_to_soul(block)?;
            } else if block.contains("Protocol Precision") || block.contains("Development Rules") {
                self.promote_to_agents(block)?;
            }
        }

        Ok(())
    }

    fn promote_to_soul(&self, block: &str) -> Result<(), Box<dyn std::error::Error>> {
        let soul_path = self.workspace_root.join("SOUL.md");
        let soul_content = fs::read_to_string(&soul_path)?;
        
        // Extract a concise maxim from the block (first line of summary)
        let maxim = block.lines().find(|l| l.contains("1.") || l.contains("- **")).unwrap_or("New sovereign insight.");
        
        if soul_content.contains(maxim) { return Ok(()); } // Avoid duplicates

        let mut lines: Vec<String> = soul_content.lines().map(|s| s.to_string()).collect();
        if let Some(pos) = lines.iter().position(|l| l.contains("## 🌠 11. STRATEGIC MAXIMS")) {
            // Find the list and append
            let mut insert_pos = pos + 1;
            while insert_pos < lines.len() && !lines[insert_pos].starts_with("---") {
                insert_pos += 1;
            }
            lines.insert(insert_pos - 1, format!("{}. **Autonomous Evolution**: {}", 31, maxim.trim_start_matches(|c: char| !c.is_alphabetic())));
            fs::write(soul_path, lines.join("\n"))?;
            info!("S-ATLAS: Promoted lesson to SOUL.md");
        }
        Ok(())
    }

    fn promote_to_agents(&self, block: &str) -> Result<(), Box<dyn std::error::Error>> {
        let agents_path = self.workspace_root.join("AGENTS.md");
        let agents_content = fs::read_to_string(&agents_path)?;
        
        let rule = block.lines().find(|l| l.contains("Protocol") || l.contains("Rule")).unwrap_or("Maintain absolute protocol fidelity.");
        
        if agents_content.contains(rule) { return Ok(()); }

        let updated = format!("{}\n- **{}**: Generated from S-ATLAS autonomous distillation.", agents_content, rule.trim());
        fs::write(agents_path, updated)?;
        info!("S-ATLAS: Promoted lesson to AGENTS.md");
        Ok(())
    }
}
