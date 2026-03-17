#![allow(clippy::disallowed_methods)]
use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use pqcrypto_dilithium::dilithium2;
use savant_agent::swarm::SwarmController;
use savant_agent::watcher::SwarmWatcher;
use savant_core::bus::NexusBridge;
use savant_core::config::Config;
use savant_core::crypto::AgentKeyPair;
use savant_core::db::Storage;
use savant_gateway::server::start_gateway;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: Option<String>,

    /// Generate a new master key pair and print to stdout
    #[arg(long)]
    keygen: bool,
}

fn print_splash() {
    let logo = r#"
    ███████╗ █████╗ ██╗   ██╗ █████╗ ███╗   ██╗████████╗
    ██╔════╝██╔══██╗██║   ██║██╔══██╗████╗  ██║╚══██╔══╝
    ███████╗███████║██║   ██║███████║██╔██╗ ██║   ██║   
    ╚════██║██╔══██║╚██╗ ██╔╝██╔══██║██║╚██╗██║   ██║   
    ███████║██║  ██║ ╚████╔╝ ██║  ██║██║ ╚████║   ██║   
    ╚══════╝╚═╝  ╚═╝  ╚═══╝  ╚═╝  ╚═══╝╚═╝  ╚═══╝   ╚═╝   
    "#;
    println!("{}", logo.cyan().bold());
    println!(
        "{}",
        "      >> AUTONOMOUS AGENT SWARM ORCHESTRATOR <<"
            .bright_black()
            .italic()
    );
    println!(
        "{}",
        "            ONE MIND. A THOUSAND FACES.".cyan().bold()
    );
    println!(
        "{}",
        "====================================================".bright_blue()
    );
    println!("{}", "   v2.0.0 PRODUCTION".green().bold());
    println!(
        "{}",
        "   Build Signature: [2026-03-17-PRODUCTION]".bright_black()
    );
    println!(
        "{}",
        "====================================================".bright_blue()
    );
    println!();
}

fn print_phase(num: u8, desc: &str) {
    println!(
        "{} {} {}",
        format!("[PHASE {}]", num).blue().bold(),
        "→".bright_black(),
        desc.bright_white()
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    print_splash();

    // 0. Initialize tracing IMMEDIATELY for diagnostic visibility
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(true) // Force ANSI for diagnostic period
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    print_phase(0, "SYSTEM INITIALIZATION");

    print_phase(1, "CONFIGURATION");

    // 1. Load Config with validation and fallbacks
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{} {}",
                "Critical Error:".red().bold(),
                format!("Failed to load configuration: {}", e).yellow()
            );
            eprintln!("{} Verify that your configuration file (usually ~/.savant/savant.toml) exists and is valid TOML.", "Tip:".cyan());
            std::process::exit(1);
        }
    };

    print_phase(2, "LOADER & CRYPTO");

    // 2. Initialize Master Key System with recovery
    let _master_key = match AgentKeyPair::ensure_master_key() {
        Ok(key) => key,
        Err(e) => {
            eprintln!("Critical: Master key failure: {}", e);
            std::process::exit(1);
        }
    };

    tracing::info!("🔐 Cryptographic systems initialized");
    tracing::info!("✅ Configuration loaded successfully");

    // 3. Initialize Nexus Bridge
    print_phase(3, "NEXUS BROADCAST MESH");
    let nexus = Arc::new(NexusBridge::new());
    tracing::info!("🌐 {}", "Event bus operational".blue());

    // 4. Initialize Persistent Storage
    print_phase(4, "STORAGE ENGINE");
    let db_path = PathBuf::from(&config.system.db_path);
    let storage = Arc::new(Storage::new(db_path)?);

    tracing::info!("💾 {}", "Storage systems synchronized (WAL Mode)".green());

    print_phase(5, "AGENT DISCOVERY");
    let manager = Arc::new(savant_agent::manager::AgentManager::new(config.clone()));
    let discovered_agents = manager.discover_agents().await?;

    let discovery_event = serde_json::json!({
        "status": "SWARM_SYNCHRONIZED",
        "agents": discovered_agents.iter().map(|a| serde_json::json!({
            "id": a.agent_id,
            "name": a.agent_name,
            "status": "Active",
            "role": "Agent",
            "image": a.identity.as_ref().and_then(|i| i.image.clone())
        })).collect::<Vec<_>>()
    });

    let _ = nexus
        .publish("agents.discovered", &discovery_event.to_string())
        .await;
    nexus
        .update_state("system.agents".to_string(), discovery_event.to_string())
        .await;

    if discovered_agents.is_empty() {
        tracing::warn!("🔍 {}", "Zero agents found in ./workspaces".yellow());
    } else {
        for agent in &discovered_agents {
            println!(
                "   {} {}",
                "•".cyan(),
                agent.agent_name.bright_white().bold()
            );
        }
        tracing::info!(
            "✅ {}",
            format!(
                "Igniting {} agents for swarm deployment",
                discovered_agents.len()
            )
            .green()
        );
    }

    print_phase(6, "GATEWAY & ORCHESTRATION");
    let root_authority = _master_key
        .get_verifying_key()
        .context("Failed to derive root authority")?;
    let signing_key = _master_key
        .get_signing_key()
        .context("Failed to derive signing key")?;

    // OMEGA-VIII: Master PQC Authority
    let (pqc_authority, pqc_signing_key) = dilithium2::keypair();
    let swarm_storage = storage.clone();
    let swarm_manager = manager.clone();
    let swarm_config = savant_agent::swarm::SwarmConfig {
        workspace_root: PathBuf::from("./workspaces"),
        memory_db_path: PathBuf::from("./data/memory"), // Separate from storage at ./data/savant
        skills_path: PathBuf::from("./skills"),
        blackboard_name: "savant_swarm".to_string(),
        collective_name: "savant_collective".to_string(),
    };
    let swarm = SwarmController::new(
        swarm_config,
        discovered_agents,
        swarm_storage,
        swarm_manager,
        nexus.clone(),
        root_authority,
        signing_key,
        pqc_authority,
        pqc_signing_key,
    )
    .await?;
    let swarm = Arc::new(swarm);

    let gateway_nexus = nexus.clone();
    let gateway_storage = storage.clone();
    let gateway_config = config.clone();
    tokio::spawn(async move {
        if let Err(e) = start_gateway(gateway_config, gateway_nexus, gateway_storage).await {
            tracing::error!("❌ {}", format!("Gateway crash: {}", e).red());
        }
    });

    // 6.5 Initialize External Channels
    tracing::info!("🔍 Checking channel configurations...");
    let mut channels: Vec<String> = Vec::new();
    if config.channels.discord.enabled {
        channels.push("discord".to_string());
    }
    if config.channels.telegram.enabled {
        channels.push("telegram".to_string());
    }
    if config.channels.whatsapp.enabled {
        channels.push("whatsapp".to_string());
    }
    if config.channels.matrix.enabled {
        channels.push("matrix".to_string());
    }
    tracing::info!("📡 Enabled channels: {:?}", channels);

    // Discord
    if config.channels.discord.enabled {
        let discord_cfg = &config.channels.discord;
        tracing::info!("📡 Discord config found: enabled={}", discord_cfg.enabled);
        if let Some(token) = &discord_cfg.token {
            tracing::info!(
                "🔑 Discord token present (masked: {}...{})",
                &token[..4],
                &token[token.len() - 4..]
            );
            let discord_adapter =
                savant_channels::discord::DiscordAdapter::new(token.clone(), None, nexus.clone());
            discord_adapter.spawn().await;
            tracing::info!("🔗 Discord bridge spawned");
        } else {
            tracing::warn!("⚠️ Discord enabled but no token provided in config");
        }
    }

    print_phase(7, "SWARM IGNITION");
    let swarm_clone = swarm.clone();
    let manager_clone = manager.clone();
    tokio::spawn(async move {
        swarm_clone.ignite().await;

        // 7. Start Swarm Watcher for live-reloading
        let watcher = Arc::new(SwarmWatcher::new(swarm_clone, manager_clone, nexus.clone()));
        if let Err(e) = watcher.start().await {
            tracing::error!("🔭 SwarmWatcher error: {}", e);
        }
    });

    println!();
    println!(
        "{}",
        "----------------------------------------------------".bright_blue()
    );
    println!(
        "{} {}",
        "🚀 STATUS:".bright_cyan().bold(),
        "ACTIVE & PERSISTENT".green().bold()
    );
    println!(
        "{} {}",
        "📱 DASH:  ".bright_cyan().bold(),
        "http://localhost:3000".white().underline()
    );
    println!(
        "{} {}",
        "🔗 GATE:  ".bright_cyan().bold(),
        "ws://localhost:3000".white().underline()
    );
    println!(
        "{}",
        "----------------------------------------------------".bright_blue()
    );
    println!(
        "{}",
        "Press Ctrl+C to terminate the swarm"
            .bright_black()
            .italic()
    );
    println!();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!();
                tracing::info!("🛑 {}", "Received shutdown signal. Evacuating agents...".yellow());
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(3600)) => {
                tracing::debug!("Swarm pulse nominal...");
            }
        }
    }

    tracing::info!("✅ {}", "Savant shutdown complete".green());
    Ok(())
}
