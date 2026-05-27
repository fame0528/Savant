use anyhow::Result;
use colored::*;
use savant_core::config::Config;
use std::path::PathBuf;

pub async fn cmd_status(config_path: &Option<String>) -> Result<()> {
    println!("{}", "=== Savant System Status ===".cyan().bold());
    println!();

    // Determine base directory from config path or use current directory
    let base_dir = config_path
        .as_ref()
        .and_then(|p| std::path::Path::new(p).parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    match Config::load_from(config_path.as_deref(), None) {
        Ok(config) => {
            println!("{} Config:    {}", "✓".green(), "Loaded".green());
            println!("  DB Path:   {}", config.system.db_path.bright_black());
        }
        Err(e) => {
            println!("{} Config:    {} ({})", "✗".red(), "Failed".red(), e);
        }
    }

    let db_path = base_dir.join("data").join("savant");
    if db_path.exists() {
        println!("{} Database:  {}", "✓".green(), "Present".green());
    } else {
        println!("{} Database:  {}", "⚠".yellow(), "Not initialized".yellow());
    }

    let memory_path = base_dir.join("data").join("memory");
    if memory_path.exists() {
        println!("{} Memory:    {}", "✓".green(), "Present".green());
    } else {
        println!("{} Memory:    {}", "⚠".yellow(), "Not initialized".yellow());
    }

    let workspaces = base_dir.join("workspaces");
    if workspaces.exists() {
        let agent_count = std::fs::read_dir(&workspaces)
            .map(|entries| entries.filter_map(|e| e.ok()).count())
            .unwrap_or(0);
        println!(
            "{} Workspaces: {} ({agent_count} agents)",
            "✓".green(),
            "Present".green()
        );
    } else {
        println!("{} Workspaces: {}", "⚠".yellow(), "Not found".yellow());
    }

    let skills = base_dir.join("skills");
    if skills.exists() {
        let skill_count = std::fs::read_dir(&skills)
            .map(|entries| entries.filter_map(|e| e.ok()).count())
            .unwrap_or(0);
        println!(
            "{} Skills:    {} ({skill_count} skills)",
            "✓".green(),
            "Present".green()
        );
    } else {
        println!("{} Skills:    {}", "⚠".yellow(), "Not found".yellow());
    }

    let config_file = base_dir.join("config").join("savant.toml");
    if config_file.exists() {
        println!("{} Config File: {}", "✓".green(), config_file.display());
    } else {
        println!("{} Config File: {}", "⚠".yellow(), "Not found".yellow());
    }

    println!();
    Ok(())
}
