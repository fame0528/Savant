use clap::Parser;
use savant_core::config::Config;
use savant_core::crypto::AgentKeyPair;
use savant_core::db::Storage;
use savant_core::bus::NexusBridge;
use savant_agent::swarm::SwarmController;
use savant_agent::watcher::SwarmWatcher;
use savant_gateway::server::start_gateway;
use std::path::PathBuf;
use std::sync::Arc;
use colored::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: Option<String>,
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
    println!("{}", "      >> AUTONOMOUS AGENT SWARM ORCHESTRATOR <<".bright_black().italic());
    println!("{}", "====================================================".bright_blue());
    println!();
}

fn print_phase(num: u8, desc: &str) {
    println!("{} {} {}", 
        format!("[PHASE {}]", num).blue().bold(),
        "→".bright_black(),
        desc.bright_white()
    );
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    print_splash();
    
    print_phase(0, "SYSTEM INITIALIZATION");
    
    // Initialize tracing with custom format for AAA feel
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
    
    print_phase(1, "LOADER & CRYPTO");
    
    // 1. Initialize Master Key System with recovery
    tracing::info!("🔐 {}", "Initializing cryptographic systems...".white());
    let _master_key = match AgentKeyPair::ensure_master_key() {
        Ok(key) => {
            tracing::info!("✅ {}", format!("Master key ready: {}...", &key.key_id[0..8]).green());
            key
        }
        Err(e) => {
            tracing::error!("❌ {}", format!("Critical: Master key failure: {}", e).red());
            std::process::exit(1);
        }
    };
    
    print_phase(2, "CONFIGURATION");
    
    // 2. Load Config with validation and fallbacks
    let config = match Config::load() {
        Ok(config) => {
            tracing::info!("✅ {}", "Configuration loaded successfully".green());
            config
        }
        Err(e) => {
            tracing::warn!("⚠️ {}", format!("No config found: {}. Using defaults.", e).yellow());
            Config::default()
        }
    };
    
    // 3. Initialize Nexus Bridge
    print_phase(3, "NEXUS BROADCAST MESH");
    let nexus = Arc::new(NexusBridge::new());
    tracing::info!("🌐 {}", "Event bus operational".blue());
    
    // 4. Initialize Persistent Storage
    print_phase(4, "STORAGE ENGINE");
    let db_path = PathBuf::from("savant.db");
    let storage = Arc::new(Storage::new(db_path));
    
    if let Err(e) = storage.init_schema() {
        tracing::error!("❌ {}", format!("Database error: {}", e).red());
    } else {
        tracing::info!("💾 {}", "Storage systems synchronized (WAL Mode)".green());
    }
    
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
    
    let _ = nexus.publish("agents.discovered", &discovery_event.to_string()).await;
    nexus.update_state("system.agents".to_string(), discovery_event.to_string()).await;
    
    if discovered_agents.is_empty() {
        tracing::warn!("🔍 {}", "Zero agents found in ./workspaces".yellow());
    } else {
        for agent in &discovered_agents {
            println!("   {} {}", "•".cyan(), agent.agent_name.bright_white().bold());
        }
        tracing::info!("✅ {}", format!("Igniting {} agents for swarm deployment", discovered_agents.len()).green());
    }
    
    print_phase(6, "GATEWAY & ORCHESTRATION");
    let swarm = Arc::new(SwarmController::new(discovered_agents, storage.clone(), manager.clone(), nexus.clone()));
    
    let gateway_nexus = nexus.clone();
    let gateway_config = config.gateway.clone();
    tokio::spawn(async move {
        if let Err(e) = start_gateway(gateway_config, gateway_nexus).await {
            tracing::error!("❌ {}", format!("Gateway crash: {}", e).red());
        }
    });

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
    println!("{}", "----------------------------------------------------".bright_blue());
    println!("{} {}", "🚀 STATUS:".bright_cyan().bold(), "ACTIVE & PERSISTENT".green().bold());
    println!("{} {}", "📱 DASH:  ".bright_cyan().bold(), "http://localhost:3000".white().underline());
    println!("{} {}", "🔗 GATE:  ".bright_cyan().bold(), "ws://localhost:8080".white().underline());
    println!("{}", "----------------------------------------------------".bright_blue());
    println!("{}", "Press Ctrl+C to terminate the swarm".bright_black().italic());
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
