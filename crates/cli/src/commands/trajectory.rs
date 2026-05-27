use anyhow::{Context, Result};
use colored::*;
use std::path::PathBuf;

/// List recorded trajectories.
#[allow(dead_code)] // Called from main.rs binary
pub async fn cmd_trajectory_list(output_dir: &str) -> Result<()> {
    let dir = PathBuf::from(output_dir);
    if !dir.exists() {
        println!(
            "{} No trajectory directory found at {}",
            "⚠".yellow(),
            dir.display()
        );
        return Ok(());
    }

    println!("{}", "=== Recorded Trajectories ===".cyan().bold());
    println!();

    let mut entries: Vec<(String, u64, String)> = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "jsonl") {
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let modified = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .map(|t| {
                    let datetime: chrono::DateTime<chrono::Utc> = t.into();
                    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_else(|_| "unknown".to_string());
            entries.push((filename, size, modified));
        }
    }

    if entries.is_empty() {
        println!("  No trajectories recorded yet.");
    } else {
        println!(
            "  {:<45} {:>10}  {}",
            "Filename".bold(),
            "Size".bold(),
            "Modified".bold()
        );
        for (name, size, modified) in &entries {
            let size_str = if *size > 1_048_576 {
                format!("{:.1} MB", *size as f64 / 1_048_576.0)
            } else if *size > 1024 {
                format!("{:.1} KB", *size as f64 / 1024.0)
            } else {
                format!("{} B", size)
            };
            println!("  {:<45} {:>10}  {}", name, size_str, modified);
        }
        println!();
        println!("  {} trajectory file(s)", entries.len());
    }

    Ok(())
}

/// Show trajectory statistics.
#[allow(dead_code)] // Called from main.rs binary
pub async fn cmd_trajectory_stats(output_dir: &str) -> Result<()> {
    let dir = PathBuf::from(output_dir);
    if !dir.exists() {
        println!(
            "{} No trajectory directory found at {}",
            "⚠".yellow(),
            dir.display()
        );
        return Ok(());
    }

    println!("{}", "=== Trajectory Statistics ===".cyan().bold());
    println!();

    let mut total_files = 0usize;
    let mut total_size = 0u64;
    let mut total_steps = 0usize;

    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "jsonl") {
            total_files += 1;
            total_size += std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            // Count steps by parsing the JSONL
            if let Ok(content) = std::fs::read_to_string(&path) {
                for line in content.lines() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(convs) = json["conversations"].as_array() {
                            total_steps += convs.len();
                        }
                    }
                }
            }
        }
    }

    let size_str = if total_size > 1_048_576 {
        format!("{:.1} MB", total_size as f64 / 1_048_576.0)
    } else if total_size > 1024 {
        format!("{:.1} KB", total_size as f64 / 1024.0)
    } else {
        format!("{} B", total_size)
    };

    println!("  Total trajectories:  {}", total_files);
    println!("  Total steps:         {}", total_steps);
    println!("  Total file size:     {}", size_str);
    if total_files > 0 {
        println!(
            "  Avg steps/traj:      {:.1}",
            total_steps as f64 / total_files as f64
        );
    }

    Ok(())
}

/// Export trajectories as ShareGPT JSONL.
#[allow(dead_code)] // Called from main.rs binary
pub async fn cmd_trajectory_export(output_dir: &str, export_path: &str) -> Result<()> {
    let dir = PathBuf::from(output_dir);
    if !dir.exists() {
        println!(
            "{} No trajectory directory found at {}",
            "⚠".yellow(),
            dir.display()
        );
        return Ok(());
    }

    let mut all_lines = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "jsonl") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                for line in content.lines() {
                    if !line.trim().is_empty() {
                        all_lines.push(line.to_string());
                    }
                }
            }
        }
    }

    if all_lines.is_empty() {
        println!("{} No trajectories to export.", "⚠".yellow());
        return Ok(());
    }

    let output = PathBuf::from(export_path);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Creating export directory {:?}", parent))?;
    }

    std::fs::write(&output, all_lines.join("\n") + "\n")
        .with_context(|| format!("Writing trajectory to {}", output.display()))?;
    println!(
        "{} Exported {} trajectories to {}",
        "✓".green(),
        all_lines.len(),
        output.display()
    );

    Ok(())
}
