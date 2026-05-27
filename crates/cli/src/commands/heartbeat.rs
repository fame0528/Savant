use anyhow::{Context, Result};
use colored::*;
use savant_core::config::Config;

pub async fn cmd_heartbeat(
    config_path: &Option<String>,
    pulse: bool,
    lens: Option<String>,
    check: bool,
) -> Result<()> {
    println!("{}", "=== Savant Heartbeat Diagnostics ===".cyan().bold());
    let _config =
        Config::load_from(config_path.as_deref(), None).unwrap_or_else(|_| Config::default());

    if pulse {
        println!("{} Connecting to Nexus bridge...", "→".bright_black());
        let nexus = savant_core::bus::NexusBridge::new();

        println!("{} Triggering manual pulse...", "→".bright_black());
        if let Some(ref l) = lens {
            println!("{} Forced Lens: {}", "→".bright_black(), l.green());
        }

        let payload = {
            let mut m = serde_json::Map::new();
            m.insert(
                "lens".to_string(),
                lens.map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null),
            );
            serde_json::Value::Object(m)
        };
        nexus
            .publish("pulse.trigger", &payload.to_string())
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        println!(
            "{} Pulse trigger successfully broadcast to the swarm.",
            "✓".green()
        );
    }

    if check {
        println!("{} Log Diversity Audit:", "→".bright_black());
        let md_path = std::path::PathBuf::from("./workspaces/workspace-savant/LEARNINGS.md");
        if md_path.exists() {
            let content =
                std::fs::read_to_string(md_path).context("Failed to read LEARNINGS.md")?;
            let mut counts = std::collections::HashMap::new();
            for line in content.lines() {
                if line.contains("### Learning") {
                    if let Some(start) = line.find("[") {
                        if let Some(end) = line.find("]") {
                            let tag = &line[start + 1..end];
                            *counts.entry(tag.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
            for (tag, count) in counts {
                println!("  • {}: {} entries", tag.cyan(), count);
            }
        } else {
            println!("  {} No LEARNINGS.md found.", "⚠".yellow());
        }
    }

    Ok(())
}
