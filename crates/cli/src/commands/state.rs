use anyhow::{Context, Result};
use colored::*;
use savant_core::config::Config;

pub async fn cmd_state(config_path: &Option<String>, inspect: bool) -> Result<()> {
    println!("{}", "=== Savant State Inspector ===".cyan().bold());
    let config =
        Config::load_from(config_path.as_deref(), None).unwrap_or_else(|_| Config::default());

    if inspect {
        println!("{} Reading Sovereign State (WAL)...", "->".bright_black());
        let substrate_path = std::path::PathBuf::from(&config.system.substrate_path);

        let state_file = substrate_path.join("DEV-SESSION-STATE.md");
        if state_file.exists() {
            let content =
                std::fs::read_to_string(&state_file).context("Failed to read session state file")?;

            if content.starts_with("---") {
                display_frontmatter_wal(&content);
            } else {
                display_legacy_wal(&content);
            }
        } else {
            println!(
                "  {} No session state file found at {:?}",
                "!".yellow(),
                state_file
            );
        }

        let context_file = substrate_path.join("CONTEXT.md");
        if context_file.exists() {
            println!();
            println!(
                "{} Reading Collective Context (Layer 2)...",
                "->".bright_black()
            );
            let content =
                std::fs::read_to_string(&context_file).context("Failed to read context file")?;
            println!("{}", content.bright_black());
        }
    }

    Ok(())
}

/// Display the new frontmatter+markdown WAL format with colored sections.
fn display_frontmatter_wal(content: &str) {
    let lines: Vec<&str> = content.lines().collect();

    // Parse frontmatter
    if lines.len() < 3 {
        println!("  {} WAL file too short", "!".red());
        return;
    }

    let mut in_frontmatter = false;
    let mut frontmatter_done = false;
    let mut frontmatter_lines = Vec::new();
    let mut body_lines = Vec::new();

    for line in &lines {
        if line.trim() == "---" {
            if !in_frontmatter {
                in_frontmatter = true;
                continue;
            } else {
                frontmatter_done = true;
                in_frontmatter = false;
                continue;
            }
        }

        if in_frontmatter {
            frontmatter_lines.push(*line);
        } else if frontmatter_done {
            body_lines.push(*line);
        }
    }

    // Display frontmatter as key-value pairs
    println!();
    println!("  {}", "=== Machine State (Frontmatter) ===".bright_blue().bold());
    for line in &frontmatter_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(colon_pos) = trimmed.find(':') {
            let key = trimmed[..colon_pos].trim().cyan();
            let value = trimmed[colon_pos + 1..].trim().bright_white();
            println!("    {} {}", key, value);
        }
    }

    // Display body sections
    println!();
    println!("  {}", "=== Human-Readable State ===".bright_green().bold());

    let body = body_lines.join("\n");
    let mut current_section = String::new();
    let mut current_content = Vec::new();

    for line in body.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            // Print previous section
            if !current_section.is_empty() {
                print_section(&current_section, &current_content);
            }
            current_section = heading.to_string();
            current_content.clear();
        } else if line.trim() == "---" {
            // Skip horizontal rules
            continue;
        } else {
            current_content.push(line.to_string());
        }
    }
    // Print last section
    if !current_section.is_empty() {
        print_section(&current_section, &current_content);
    }
}

fn print_section(heading: &str, content: &[String]) {
    println!();
    println!("    {}", heading.bright_yellow().bold());
    for line in content {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "(empty)" || trimmed == "(none)" {
            println!("      {}", trimmed.bright_black());
        } else if trimmed.starts_with('|') && !trimmed.starts_with("|---") {
            // Table row
            println!("      {}", trimmed.bright_white());
        } else if trimmed.starts_with("- ") {
            // Bullet list
            println!("      {}", trimmed.bright_white());
        } else if let Some(pos) = trimmed.find(". ") {
            if trimmed[..pos].chars().all(|c| c.is_ascii_digit()) {
                // Numbered list
                println!("      {}", trimmed.bright_white());
            } else {
                println!("      {}", trimmed.bright_white());
            }
        } else {
            println!("      {}", trimmed.bright_white());
        }
    }
}

/// Display the legacy JSON WAL format.
fn display_legacy_wal(content: &str) {
    println!(
        "  {} Legacy JSON WAL format detected",
        "!".yellow()
    );
    println!(
        "  {} The file will be migrated to frontmatter format on next agent pulse",
        "i".bright_black()
    );
    println!();
    // Show first 20 lines of the raw content
    for (i, line) in content.lines().enumerate() {
        if i >= 20 {
            println!("  {} ... (truncated)", "...".bright_black());
            break;
        }
        println!("  {}", line.bright_black());
    }
}
