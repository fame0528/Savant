use anyhow::{Context, Result};
use colored::*;
use savant_core::config::Config;
use std::path::PathBuf;

pub async fn cmd_backup(
    config_path: &Option<String>,
    output: &str,
    include_memory: bool,
) -> Result<()> {
    println!("{}", "=== Savant Database Backup ===".cyan().bold());
    println!();

    let config =
        Config::load_from(config_path.as_deref(), None).unwrap_or_else(|_| Config::default());

    let db_path = PathBuf::from(&config.system.db_path);
    let output_path = PathBuf::from(output);

    println!("{} Database: {:?}", "→".bright_black(), db_path);
    println!("{} Output:   {:?}", "→".bright_black(), output_path);

    if !db_path.exists() {
        eprintln!(
            "{} Database path does not exist: {:?}",
            "Error:".red().bold(),
            db_path
        );
        std::process::exit(1);
    }

    println!("{} Creating backup...", "→".bright_black());

    if output_path.exists() {
        std::fs::remove_dir_all(&output_path)
            .context("Failed to clean existing backup directory")?;
    }

    savant_core::utils::io::copy_dir_recursive(&db_path, &output_path)
        .context("Failed to copy database files")?;

    if include_memory {
        let memory_path = PathBuf::from("./data/memory");
        if memory_path.exists() {
            let memory_backup = output_path.join("memory");
            println!("{} Including memory database...", "→".bright_black());
            savant_core::utils::io::copy_dir_recursive(&memory_path, &memory_backup)
                .context("Failed to copy memory database")?;
        }
    }

    println!();
    println!("{}", "✅ Backup completed successfully".green().bold());
    Ok(())
}
