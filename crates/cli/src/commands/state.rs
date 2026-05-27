use anyhow::{Context, Result};
use colored::*;
use savant_core::config::Config;

pub async fn cmd_state(config_path: &Option<String>, inspect: bool) -> Result<()> {
    println!("{}", "=== Savant State Inspector ===".cyan().bold());
    let config =
        Config::load_from(config_path.as_deref(), None).unwrap_or_else(|_| Config::default());

    if inspect {
        println!("{} Reading Sovereign State (WAL)...", "→".bright_black());
        let substrate_path = std::path::PathBuf::from(&config.system.substrate_path);

        let state_file = substrate_path.join("DEV-SESSION-STATE.md");
        if state_file.exists() {
            println!(
                "  {} File: {:?}",
                "•".cyan(),
                state_file.file_name().unwrap_or_default()
            );
            let content =
                std::fs::read_to_string(state_file).context("Failed to read session state file")?;
            println!("{}", content.bright_black());
        } else {
            println!(
                "  {} No session state file found at {:?}",
                "⚠".yellow(),
                state_file
            );
        }

        let context_file = substrate_path.join("CONTEXT.md");
        if context_file.exists() {
            println!();
            println!(
                "{} Reading Collective Context (Layer 2)...",
                "→".bright_black()
            );
            println!(
                "  {} File: {:?}",
                "•".cyan(),
                context_file.file_name().unwrap_or_default()
            );
            let content =
                std::fs::read_to_string(context_file).context("Failed to read context file")?;
            println!("{}", content.bright_black());
        }
    }

    Ok(())
}
