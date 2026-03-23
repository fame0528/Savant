//! Daily Operational Logs
//!
//! Append-only logs per agent per day. Loaded on session start for immediate
//! orientation. Format: Markdown (human-readable, token-efficient).
//!
//! # Structure
//! ```text
//! workspaces/agents/<agent>/memory/
//! └── 2026-03-19.md
//! ```
//!
//! # Token Budget
//! Capped at 500 tokens. Loaded as first context element after system prompt.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tracing::debug;

/// Daily operational log for an agent.
pub struct DailyLog {
    agent_workspace: PathBuf,
    agent_name: String,
}

/// An entry in the daily log.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub category: String,
    pub content: String,
    pub priority: LogPriority,
}

/// Log entry priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogPriority {
    Info,
    Success,
    Warning,
    Error,
    Blocker,
}

impl std::fmt::Display for LogPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogPriority::Info => write!(f, "INFO"),
            LogPriority::Success => write!(f, "SUCCESS"),
            LogPriority::Warning => write!(f, "WARNING"),
            LogPriority::Error => write!(f, "ERROR"),
            LogPriority::Blocker => write!(f, "BLOCKER"),
        }
    }
}

impl DailyLog {
    /// Creates a new daily log for the given agent workspace.
    pub fn new(agent_workspace: PathBuf, agent_name: String) -> Self {
        Self {
            agent_workspace,
            agent_name,
        }
    }

    /// Returns the path to today's log file.
    pub fn today_path(&self) -> PathBuf {
        let date = Self::today_date();
        self.log_path(&date)
    }

    /// Returns the path to a specific date's log file.
    pub fn log_path(&self, date: &str) -> PathBuf {
        self.agent_workspace
            .join("memory")
            .join(format!("{}.md", date))
    }

    /// Gets today's date as YYYY-MM-DD.
    pub fn today_date() -> String {
        let now = std::time::SystemTime::now();
        let duration = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        // Simple date calculation (days since epoch)
        let days = secs / 86400;
        let (year, month, day) = Self::days_to_ymd(days as i64);
        format!("{:04}-{:02}-{:02}", year, month, day)
    }

    /// Appends an entry to today's log.
    pub fn append(&self, entry: &LogEntry) -> Result<(), std::io::Error> {
        let path = self.today_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let now = Self::current_time();

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        // If file is new, write header
        if file.metadata()?.len() == 0 {
            writeln!(file, "# {} — {}", Self::today_date(), self.agent_name)?;
            writeln!(file)?;
        }

        writeln!(
            file,
            "## {} — {}\n- {}\n",
            now, entry.category, entry.content
        )?;

        debug!("Appended to daily log: {:?}", path);
        Ok(())
    }

    /// Reads today's log content. Returns empty string if no log exists.
    pub fn read_today(&self) -> Result<String, std::io::Error> {
        let path = self.today_path();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            // Cap at ~2000 bytes, aligned to char boundary to prevent UTF-8 panics
            if content.len() > 2000 {
                let mut start = content.len() - 2000;
                while start > 0 && !content.is_char_boundary(start) {
                    start -= 1;
                }
                Ok(content[start..].to_string())
            } else {
                Ok(content)
            }
        } else {
            Ok(String::new())
        }
    }

    /// Reads a specific date's log.
    pub fn read_date(&self, date: &str) -> Result<String, std::io::Error> {
        let path = self.log_path(date);
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            // Cap at ~2000 bytes, aligned to char boundary to prevent UTF-8 panics
            if content.len() > 2000 {
                let mut start = content.len() - 2000;
                while start > 0 && !content.is_char_boundary(start) {
                    start -= 1;
                }
                Ok(content[start..].to_string())
            } else {
                Ok(content)
            }
        } else {
            Ok(String::new())
        }
    }

    /// Lists all log files for this agent.
    pub fn list_logs(&self) -> Vec<String> {
        let memory_dir = self.agent_workspace.join("memory");
        if !memory_dir.exists() {
            return Vec::new();
        }

        let mut logs = Vec::new();
        if let Ok(entries) = fs::read_dir(&memory_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md") && name.len() == 13 {
                    // YYYY-MM-DD.md = 13 chars
                    logs.push(name.replace(".md", ""));
                }
            }
        }
        logs.sort();
        logs
    }

    /// Rotates logs older than retention_days.
    pub fn rotate(&self, retention_days: u32) -> Result<usize, std::io::Error> {
        let logs = self.list_logs();
        let today = Self::today_date();
        let mut rotated = 0;

        for date in logs {
            let age = Self::days_between(&date, &today);
            if age > retention_days as i64 {
                let path = self.log_path(&date);
                let archive_dir = self.agent_workspace.join("memory").join("archive");
                fs::create_dir_all(&archive_dir)?;
                let archive_path = archive_dir.join(format!("{}.md", date));
                fs::rename(&path, &archive_path)?;
                rotated += 1;
                debug!("Archived daily log: {}", date);
            }
        }

        Ok(rotated)
    }

    fn current_time() -> String {
        let now = std::time::SystemTime::now();
        let duration = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs() % 86400;
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{:02}:{:02}", hours, mins)
    }

    fn days_to_ymd(days: i64) -> (i32, u32, u32) {
        // Use chrono for correct date arithmetic (handles leap years, centuries, 400-year rules)
        use chrono::Datelike;
        chrono::NaiveDate::from_num_days_from_ce_opt(days as i32 + 719163)
            .map(|d| (d.year(), d.month(), d.day()))
            .unwrap_or((1970, 1, 1))
    }

    fn days_between(from: &str, to: &str) -> i64 {
        Self::date_to_days(to) - Self::date_to_days(from)
    }

    fn date_to_days(date: &str) -> i64 {
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map(|d| {
                d.signed_duration_since(chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
                    .num_days()
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_today_date_format() {
        let date = DailyLog::today_date();
        assert_eq!(date.len(), 10);
        assert!(date.contains('-'));
    }

    #[test]
    fn test_append_and_read() {
        let temp = tempfile::tempdir().unwrap();
        let log = DailyLog::new(temp.path().to_path_buf(), "TestAgent".to_string());

        let entry = LogEntry {
            timestamp: "09:00".to_string(),
            category: "Session Started".to_string(),
            content: "Resumed task: Docker networking".to_string(),
            priority: LogPriority::Info,
        };

        log.append(&entry).unwrap();

        let content = log.read_today().unwrap();
        assert!(content.contains("TestAgent"));
        assert!(content.contains("Docker networking"));
    }

    #[test]
    fn test_read_nonexistent_returns_empty() {
        let temp = tempfile::tempdir().unwrap();
        let log = DailyLog::new(temp.path().to_path_buf(), "TestAgent".to_string());
        let content = log.read_today().unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn test_multiple_append() {
        let temp = tempfile::tempdir().unwrap();
        let log = DailyLog::new(temp.path().to_path_buf(), "TestAgent".to_string());

        for i in 0..5 {
            let entry = LogEntry {
                timestamp: format!("{}0:00", i + 9),
                category: format!("Task {}", i),
                content: format!("Step {}", i),
                priority: LogPriority::Info,
            };
            log.append(&entry).unwrap();
        }

        let content = log.read_today().unwrap();
        assert!(content.contains("Task 0"));
        assert!(content.contains("Task 4"));
    }

    #[test]
    fn test_list_logs() {
        let temp = tempfile::tempdir().unwrap();
        let log = DailyLog::new(temp.path().to_path_buf(), "TestAgent".to_string());

        // Create some logs
        let entry = LogEntry {
            timestamp: "09:00".to_string(),
            category: "Test".to_string(),
            content: "Test content".to_string(),
            priority: LogPriority::Info,
        };
        log.append(&entry).unwrap();

        let logs = log.list_logs();
        assert_eq!(logs.len(), 1);
    }
}
