use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use savant_core::error::SavantError;
use savant_core::learning::{EmergentLearning, LearningCategory};

/// LEARNINGS.md Parser
///
/// Converts free-form LEARNINGS.md entries into structured LEARNINGS.jsonl format.
/// This enables the dashboard to display reflections while preserving the original
/// free-form writing style.
pub struct LearningsParser {
    workspace_path: PathBuf,
}

impl LearningsParser {
    pub fn new(workspace_path: PathBuf) -> Self {
        Self { workspace_path }
    }

    /// Parses LEARNINGS.md and converts new entries to JSONL format.
    /// Returns the number of new entries parsed.
    pub fn parse_and_convert(&self, agent_id: &str) -> Result<usize, SavantError> {
        let md_path = self.workspace_path.join("LEARNINGS.md");
        let jsonl_path = self.workspace_path.join("LEARNINGS.jsonl");

        if !md_path.exists() {
            debug!("No LEARNINGS.md found at {:?}", md_path);
            return Ok(0);
        }

        let md_content = fs::read_to_string(&md_path).map_err(SavantError::IoError)?;

        // Parse entries from LEARNINGS.md
        let entries = self.parse_entries(&md_content, agent_id)?;

        // Get existing JSONL entries to avoid duplicates
        let existing_timestamps = self.get_existing_timestamps(&jsonl_path)?;

        // Filter new entries
        let new_entries: Vec<EmergentLearning> = entries
            .into_iter()
            .filter(|entry| !existing_timestamps.contains(&entry.timestamp))
            .collect();

        if new_entries.is_empty() {
            return Ok(0);
        }

        // Append new entries to JSONL
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&jsonl_path)
            .map_err(SavantError::IoError)?;

        for entry in &new_entries {
            let json =
                serde_json::to_string(entry).map_err(|e| SavantError::SerializationError(e))?;
            use std::io::Write;
            writeln!(file, "{}", json).map_err(SavantError::IoError)?;
        }

        info!(
            "📚 Parsed {} new learning entries from LEARNINGS.md → LEARNINGS.jsonl",
            new_entries.len()
        );

        Ok(new_entries.len())
    }

    /// Parses LEARNINGS.md entries into EmergentLearning structs.
    fn parse_entries(
        &self,
        content: &str,
        agent_id: &str,
    ) -> Result<Vec<EmergentLearning>, SavantError> {
        let mut entries = Vec::new();

        // Split by "### Learning (" to find entries
        let parts: Vec<&str> = content.split("### Learning (").collect();

        for part in parts.iter().skip(1) {
            // Extract timestamp
            let _timestamp = self.extract_timestamp(part);
            let _timestamp = _timestamp.unwrap_or_else(|| Utc::now().to_rfc3339());

            // Extract content (everything after the timestamp line)
            let content_text = self.extract_content(part);

            if content_text.trim().is_empty() {
                continue;
            }

            // Categorize based on content
            let category = self.categorize(&content_text);

            // Calculate significance
            let significance = self.calculate_significance(&content_text);

            let entry =
                EmergentLearning::new(agent_id.to_string(), category, content_text, significance);

            entries.push(entry);
        }

        Ok(entries)
    }

    /// Extracts timestamp from entry text.
    fn extract_timestamp(&self, text: &str) -> Option<String> {
        // Format: 2026-03-12 17:03:30.213428200 UTC
        let line = text.lines().next()?;

        // Try to parse various timestamp formats
        if let Some(end) = line.find(" UTC)") {
            let ts_str = &line[..end];
            // Convert to RFC3339 format
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S%.f") {
                return Some(dt.and_utc().to_rfc3339());
            }
        }

        None
    }

    /// Extracts content text from entry.
    fn extract_content(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();

        // Find the content after the timestamp line
        let mut content_lines = Vec::new();
        let mut started = false;

        for line in lines {
            if !started {
                // Skip until we find the closing parenthesis or newline
                if line.contains("UTC)") || line.trim().is_empty() {
                    started = true;
                    continue;
                }
            }
            if started {
                content_lines.push(line);
            }
        }

        content_lines.join("\n").trim().to_string()
    }

    /// Categorizes content based on keywords.
    fn categorize(&self, content: &str) -> LearningCategory {
        let lower = content.to_lowercase();

        if lower.contains("error") || lower.contains("bug") || lower.contains("fix") {
            LearningCategory::Error
        } else if lower.contains("protocol")
            || lower.contains("procedure")
            || lower.contains("rule")
        {
            LearningCategory::Protocol
        } else {
            LearningCategory::Insight
        }
    }

    /// Calculates significance score (0-10) based on content.
    fn calculate_significance(&self, content: &str) -> u8 {
        let mut score: u8 = 5; // Base score

        // Length bonus
        if content.len() > 500 {
            score += 1;
        }
        if content.len() > 1000 {
            score += 1;
        }

        // Keyword bonus
        let lower = content.to_lowercase();
        if lower.contains("strategic") || lower.contains("critical") {
            score += 1;
        }
        if lower.contains("empire") || lower.contains("sovereign") {
            score += 1;
        }
        if lower.contains("breakthrough") || lower.contains("revelation") {
            score += 1;
        }

        score.min(10)
    }

    /// Gets existing timestamps from JSONL file.
    fn get_existing_timestamps(&self, jsonl_path: &Path) -> Result<Vec<String>, SavantError> {
        let mut timestamps = Vec::new();

        if !jsonl_path.exists() {
            return Ok(timestamps);
        }

        let content = fs::read_to_string(jsonl_path).map_err(SavantError::IoError)?;

        for line in content.lines() {
            if let Ok(entry) = serde_json::from_str::<EmergentLearning>(line) {
                timestamps.push(entry.timestamp);
            }
        }

        Ok(timestamps)
    }
}
