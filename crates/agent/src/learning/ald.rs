use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use tracing::{info, warn};

/// Protocol S-ATLAS: Autonomous Lesson Distillation (ALD)
/// Distills raw learnings into core sovereign SOUL files.
pub struct ALDEngine {
    workspace_root: PathBuf,
}

impl ALDEngine {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Run a distillation cycle starting from the provided watermark.
    /// Returns the new watermark and a boolean indicating if a 'High Density Burst' was detected.
    pub fn distill(&self, watermark: u64) -> Result<(u64, bool), Box<dyn std::error::Error>> {
        let learnings_path = self.workspace_root.join("LEARNINGS.md");
        if !learnings_path.exists() {
            return Ok((0, false));
        }

        let mut file = fs::File::open(&learnings_path)?;
        let file_len = file.metadata()?.len();
        
        // Seek to watermark (with safety check)
        let actual_offset = if watermark > file_len {
            warn!("ALD: Watermark exceeds file length. Resetting to 0.");
            0
        } else {
            watermark
        };
        file.seek(SeekFrom::Start(actual_offset))?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;

        if content.is_empty() {
            return Ok((file_len, false));
        }

        let blocks: Vec<&str> = content.split("### Learning").collect();
        let mut priority_hits = 0;

        // AAA: Heuristic - Only promote deep reflections (>500 chars or specific keywords)
        // skip(1) if we started from 0 (the first segment is usually preamble)
        let start_idx = if actual_offset == 0 { 1 } else { 0 };
        for block in blocks.iter().skip(start_idx) {
            let is_strategic = block.contains("[STRATEGY]") || block.contains("[STRATEGIC]") || block.contains("Strategic Insight");
            let is_engineering = block.contains("[ENGINEERING]") || block.contains("Protocol Precision");

            if is_strategic {
                self.promote_to_soul(block)?;
                priority_hits += 2;
            } else if is_engineering {
                self.promote_to_agents(block)?;
                priority_hits += 1;
            }
        }

        let burst_detected = priority_hits >= 3;
        Ok((file_len, burst_detected))
    }

    fn promote_to_soul(&self, block: &str) -> Result<(), Box<dyn std::error::Error>> {
        let soul_path = self.workspace_root.join("SOUL.md");
        let soul_content = fs::read_to_string(&soul_path)?;

        // Extract a concise maxim from the block (first line of summary)
        let maxim = block
            .lines()
            .find(|l| l.contains("1.") || l.contains("- **"))
            .unwrap_or("New sovereign insight.");

        if soul_content.contains(maxim) {
            return Ok(());
        } // Avoid duplicates

        let mut lines: Vec<String> = soul_content.lines().map(|s| s.to_string()).collect();
        if let Some(pos) = lines
            .iter()
            .position(|l| l.contains("## 🌠 11. STRATEGIC MAXIMS"))
        {
            // Find the list and append
            let mut insert_pos = pos + 1;
            while insert_pos < lines.len() && !lines[insert_pos].starts_with("---") {
                insert_pos += 1;
            }
            lines.insert(
                insert_pos - 1,
                format!(
                    "{}. **Autonomous Evolution**: {}",
                    31,
                    maxim.trim_start_matches(|c: char| !c.is_alphabetic())
                ),
            );
            fs::write(soul_path, lines.join("\n"))?;
            info!("S-ATLAS: Promoted lesson to SOUL.md");
        }
        Ok(())
    }

    fn promote_to_agents(&self, block: &str) -> Result<(), Box<dyn std::error::Error>> {
        let agents_path = self.workspace_root.join("AGENTS.md");
        let agents_content = fs::read_to_string(&agents_path)?;

        let rule = block
            .lines()
            .find(|l| l.contains("Protocol") || l.contains("Rule"))
            .unwrap_or("Maintain absolute protocol fidelity.");

        if agents_content.contains(rule) {
            return Ok(());
        }

        let updated = format!(
            "{}\n- **{}**: Generated from S-ATLAS autonomous distillation.",
            agents_content,
            rule.trim()
        );
        fs::write(agents_path, updated)?;
        info!("S-ATLAS: Promoted lesson to AGENTS.md");
        Ok(())
    }
}
