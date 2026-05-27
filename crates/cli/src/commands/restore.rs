use anyhow::{Context, Result};
use colored::*;
use savant_core::config::Config;
use std::path::PathBuf;

pub async fn cmd_restore(config_path: &Option<String>, input: &str) -> Result<()> {
    println!("{}", "=== Savant Database Restore ===".cyan().bold());
    println!();

    let input_path = PathBuf::from(input);
    if !input_path.exists() {
        eprintln!(
            "{} Backup path does not exist: {:?}",
            "Error:".red().bold(),
            input_path
        );
        std::process::exit(1);
    }

    let config =
        Config::load_from(config_path.as_deref(), None).unwrap_or_else(|_| Config::default());

    let db_path = PathBuf::from(&config.system.db_path);

    println!("{} Backup:   {:?}", "→".bright_black(), input_path);
    println!("{} Target:   {:?}", "→".bright_black(), db_path);

    if db_path.exists() {
        let pre_restore_backup = db_path.with_extension("pre-restore");
        println!(
            "{} Backing up existing database to {:?}...",
            "→".bright_black(),
            pre_restore_backup
        );
        savant_core::utils::io::copy_dir_recursive(&db_path, &pre_restore_backup)
            .context("Failed to backup existing database")?;
    }

    println!("{} Restoring database...", "→".bright_black());

    if db_path.exists() {
        std::fs::remove_dir_all(&db_path).context("Failed to clean existing database")?;
    }

    savant_core::utils::io::copy_dir_recursive(&input_path, &db_path)
        .context("Failed to restore database files")?;

    let memory_backup = input_path.join("memory");
    if memory_backup.exists() {
        let memory_path = PathBuf::from("./data/memory");
        println!("{} Restoring memory database...", "→".bright_black());
        if memory_path.exists() {
            std::fs::remove_dir_all(&memory_path)
                .context("Failed to clean existing memory database")?;
        }
        savant_core::utils::io::copy_dir_recursive(&memory_backup, &memory_path)
            .context("Failed to restore memory database")?;
    }

    println!();
    println!("{}", "✅ Restore completed successfully".green().bold());
    Ok(())
}
