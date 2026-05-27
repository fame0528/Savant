use anyhow::{Context, Result};
use colored::*;
use savant_core::config::Config;

pub async fn cmd_list_agents(config_path: &Option<String>) -> Result<()> {
    println!("{}", "=== Discovered Agents ===".cyan().bold());
    println!();

    let config =
        Config::load_from(config_path.as_deref(), None).unwrap_or_else(|_| Config::default());

    let manager = savant_agent::manager::AgentManager::new(config);
    let agents = manager
        .discover_agents()
        .await
        .context("Failed to discover agents")?;

    if agents.is_empty() {
        println!("{}", "No agents found in ./workspaces".yellow());
    } else {
        println!("Found {} agent(s):", agents.len());
        println!();
        for agent in &agents {
            println!(
                "  {} {} ({})",
                "•".cyan(),
                agent.agent_name.bright_white().bold(),
                agent.agent_id.bright_black()
            );
            if let Some(ref identity) = agent.identity {
                if !identity.soul.is_empty() {
                    let preview: String = identity.soul.chars().take(80).collect();
                    println!("    {}", preview.bright_black());
                }
            }
        }
    }

    Ok(())
}
