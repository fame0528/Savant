#![allow(clippy::disallowed_methods)]
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
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

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the Savant agent swarm orchestrator (default)
    Start,

    /// Test a skill from a SKILL.md file
    TestSkill {
        /// Path to the SKILL.md file
        #[arg(short, long)]
        skill_path: String,

        /// JSON input payload for the skill
        #[arg(short, long, default_value = "{}")]
        input: String,

        /// Timeout in seconds
        #[arg(short, long, default_value = "30")]
        timeout: u64,
    },

    /// Backup the Savant database to a file
    Backup {
        /// Output path for the backup file
        #[arg(short, long)]
        output: String,

        /// Backup memory database (default: main database)
        #[arg(long)]
        include_memory: bool,
    },

    /// Restore the Savant database from a backup file
    Restore {
        /// Input path for the backup file
        #[arg(short, long)]
        input: String,
    },

    /// List discovered agents
    ListAgents,

    /// Display system status and health
    Status,
}

fn print_splash() {
    let logo = r#"
    ███████╗ █████╗ ██╗   ██╗ █████╗ ███╗   ██╗████████╗
    ██╔════╝██╔══██║██║   ██║██╔══██╗████╗  ██║╚══██╔══╝
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

    // Dynamic build timestamp using std::time (no extra dependency)
    let build_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            let days = secs / 86400;
            let years = 1970 + days / 365;
            years.to_string()
        })
        .unwrap_or_else(|_| "unknown".to_string());
    println!(
        "{}",
        format!("   Build: {} (runtime)", build_time).bright_black()
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

// ============================================================================
// Subcommand: Start (original swarm orchestrator)
// ============================================================================

async fn cmd_start(config_path: Option<String>) -> Result<()> {
    print_splash();

    print_phase(0, "SYSTEM INITIALIZATION");
    print_phase(1, "CONFIGURATION");

    let config = match Config::load_from(config_path.as_deref()) {
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

    let _master_key = match AgentKeyPair::ensure_master_key() {
        Ok(key) => key,
        Err(e) => {
            eprintln!("Critical: Master key failure: {}", e);
            std::process::exit(1);
        }
    };

    tracing::info!("🔐 Cryptographic systems initialized");
    tracing::info!("✅ Configuration loaded successfully");

    print_phase(3, "NEXUS BROADCAST MESH");
    let nexus = Arc::new(NexusBridge::new());
    tracing::info!("🌐 {}", "Event bus operational".blue());

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

    let (pqc_authority, pqc_signing_key) = dilithium2::keypair();
    let swarm_storage = storage.clone();
    let swarm_manager = manager.clone();
    let swarm_config = savant_agent::swarm::SwarmConfig {
        workspace_root: PathBuf::from("./workspaces"),
        memory_db_path: PathBuf::from("./data/memory"),
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

    if config.channels.discord.enabled {
        let discord_cfg = &config.channels.discord;
        tracing::info!("📡 Discord config found: enabled={}", discord_cfg.enabled);
        if let Some(token) = &discord_cfg.token {
            tracing::info!(
                "🔑 Discord token present (masked: {}...{})",
                &token[..token.len().min(4)],
                &token[token.len().saturating_sub(4)..]
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

// ============================================================================
// Subcommand: test-skill
// ============================================================================

async fn cmd_test_skill(skill_path: &str, input: &str, timeout_secs: u64) -> Result<()> {
    println!("{}", "=== Savant Skill Tester ===".cyan().bold());
    println!();

    let path = PathBuf::from(skill_path);
    if !path.exists() {
        eprintln!("{} Skill file not found: {}", "Error:".red().bold(), skill_path);
        std::process::exit(1);
    }

    // Parse the skill manifest
    println!("{} Loading skill from: {}", "→".bright_black(), skill_path);
    let mut registry = savant_skills::parser::SkillRegistry::new();
    registry
        .load_skill_from_file(&path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load skill: {}", e))?;

    let skill_name = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("{} Skill name: {}", "→".bright_black(), skill_name.green());
    println!();

    // Parse input
    let input_value: serde_json::Value = serde_json::from_str(input)
        .unwrap_or_else(|_| serde_json::json!({"raw": input}));

    println!("{} Input: {}", "→".bright_black(), serde_json::to_string_pretty(&input_value).unwrap_or_default());
    println!();

    // Find and execute the tool
    if let Some(tool) = registry.tools.values().next() {
        println!("{} Executing skill...", "→".bright_black());

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tool.execute(input_value),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                println!();
                println!("{}", "=== Output ===".green().bold());
                println!("{}", output);
                println!();
                println!("{}", "✅ Skill test PASSED".green().bold());
            }
            Ok(Err(e)) => {
                println!();
                println!("{}", "=== Error ===".red().bold());
                println!("{}", e);
                println!();
                println!("{}", "❌ Skill test FAILED".red().bold());
                std::process::exit(1);
            }
            Err(_) => {
                println!();
                println!("{}", format!("❌ Skill test TIMED OUT after {}s", timeout_secs).red().bold());
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("{} No tools found in skill manifest", "Error:".red().bold());
        std::process::exit(1);
    }

    Ok(())
}

// ============================================================================
// Subcommand: backup
// ============================================================================

async fn cmd_backup(config_path: &Option<String>, output: &str, include_memory: bool) -> Result<()> {
    println!("{}", "=== Savant Database Backup ===".cyan().bold());
    println!();

    let config = Config::load_from(config_path.as_deref())
        .unwrap_or_else(|_| Config::default());

    let db_path = PathBuf::from(&config.system.db_path);
    let output_path = PathBuf::from(output);

    println!("{} Database: {:?}", "→".bright_black(), db_path);
    println!("{} Output:   {:?}", "→".bright_black(), output_path);

    // Create the backup by copying the database directory
    if !db_path.exists() {
        eprintln!("{} Database path does not exist: {:?}", "Error:".red().bold(), db_path);
        std::process::exit(1);
    }

    // Create backup archive
    println!("{} Creating backup...", "→".bright_black());

    // Copy all files from the database directory to the output
    if output_path.exists() {
        std::fs::remove_dir_all(&output_path)
            .context("Failed to clean existing backup directory")?;
    }

    copy_dir_recursive(&db_path, &output_path)
        .context("Failed to copy database files")?;

    // Optionally backup memory database
    if include_memory {
        let memory_path = PathBuf::from("./data/memory");
        if memory_path.exists() {
            let memory_backup = output_path.join("memory");
            println!("{} Including memory database...", "→".bright_black());
            copy_dir_recursive(&memory_path, &memory_backup)
                .context("Failed to copy memory database")?;
        }
    }

    println!();
    println!("{}", "✅ Backup completed successfully".green().bold());
    Ok(())
}

// ============================================================================
// Subcommand: restore
// ============================================================================

async fn cmd_restore(config_path: &Option<String>, input: &str) -> Result<()> {
    println!("{}", "=== Savant Database Restore ===".cyan().bold());
    println!();

    let input_path = PathBuf::from(input);
    if !input_path.exists() {
        eprintln!("{} Backup path does not exist: {:?}", "Error:".red().bold(), input_path);
        std::process::exit(1);
    }

    let config = Config::load_from(config_path.as_deref())
        .unwrap_or_else(|_| Config::default());

    let db_path = PathBuf::from(&config.system.db_path);

    println!("{} Backup:   {:?}", "→".bright_black(), input_path);
    println!("{} Target:   {:?}", "→".bright_black(), db_path);

    // Backup existing database before restore
    if db_path.exists() {
        let pre_restore_backup = db_path.with_extension("pre-restore");
        println!("{} Backing up existing database to {:?}...", "→".bright_black(), pre_restore_backup);
        copy_dir_recursive(&db_path, &pre_restore_backup)
            .context("Failed to backup existing database")?;
    }

    // Restore from backup
    println!("{} Restoring database...", "→".bright_black());

    if db_path.exists() {
        std::fs::remove_dir_all(&db_path)
            .context("Failed to clean existing database")?;
    }

    copy_dir_recursive(&input_path, &db_path)
        .context("Failed to restore database files")?;

    // Check for memory backup
    let memory_backup = input_path.join("memory");
    if memory_backup.exists() {
        let memory_path = PathBuf::from("./data/memory");
        println!("{} Restoring memory database...", "→".bright_black());
        if memory_path.exists() {
            std::fs::remove_dir_all(&memory_path)
                .context("Failed to clean existing memory database")?;
        }
        copy_dir_recursive(&memory_backup, &memory_path)
            .context("Failed to restore memory database")?;
    }

    println!();
    println!("{}", "✅ Restore completed successfully".green().bold());
    Ok(())
}

// ============================================================================
// Subcommand: list-agents
// ============================================================================

async fn cmd_list_agents(config_path: &Option<String>) -> Result<()> {
    println!("{}", "=== Discovered Agents ===".cyan().bold());
    println!();

    let config = Config::load_from(config_path.as_deref())
        .unwrap_or_else(|_| Config::default());

    let manager = savant_agent::manager::AgentManager::new(config);
    let agents = manager.discover_agents().await?;

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

// ============================================================================
// Subcommand: status
// ============================================================================

async fn cmd_status(config_path: &Option<String>) -> Result<()> {
    println!("{}", "=== Savant System Status ===".cyan().bold());
    println!();

    // Check config
    match Config::load_from(config_path.as_deref()) {
        Ok(config) => {
            println!("{} Config:    {}", "✓".green(), "Loaded".green());
            println!("  DB Path:   {}", config.system.db_path.bright_black());
        }
        Err(e) => {
            println!("{} Config:    {} ({})", "✗".red(), "Failed".red(), e);
        }
    }

    // Check database
    let db_path = PathBuf::from("./data/savant");
    if db_path.exists() {
        println!("{} Database:  {}", "✓".green(), "Present".green());
    } else {
        println!("{} Database:  {}", "⚠".yellow(), "Not initialized".yellow());
    }

    // Check memory
    let memory_path = PathBuf::from("./data/memory");
    if memory_path.exists() {
        println!("{} Memory:    {}", "✓".green(), "Present".green());
    } else {
        println!("{} Memory:    {}", "⚠".yellow(), "Not initialized".yellow());
    }

    // Check workspaces
    let workspaces = PathBuf::from("./workspaces");
    if workspaces.exists() {
        let agent_count = std::fs::read_dir(&workspaces)
            .map(|entries| entries.filter_map(|e| e.ok()).count())
            .unwrap_or(0);
        println!("{} Workspaces: {} ({})", "✓".green(), "Present".green(), format!("{} agents", agent_count));
    } else {
        println!("{} Workspaces: {}", "⚠".yellow(), "Not found".yellow());
    }

    // Check skills
    let skills = PathBuf::from("./skills");
    if skills.exists() {
        let skill_count = std::fs::read_dir(&skills)
            .map(|entries| entries.filter_map(|e| e.ok()).count())
            .unwrap_or(0);
        println!("{} Skills:    {} ({})", "✓".green(), "Present".green(), format!("{} skills", skill_count));
    } else {
        println!("{} Skills:    {}", "⚠".yellow(), "Not found".yellow());
    }

    // Check config file
    let config_file = PathBuf::from("./config/savant.toml");
    if config_file.exists() {
        println!("{} Config File: {}", "✓".green(), config_file.display());
    } else {
        println!("{} Config File: {}", "⚠".yellow(), "Not found".yellow());
    }

    println!();
    Ok(())
}

// ============================================================================
// Utility: Recursive directory copy
// ============================================================================

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in walkdir::WalkDir::new(src) {
        let entry = entry.context("Failed to read directory entry")?;
        let src_path = entry.path();
        let relative = src_path.strip_prefix(src).context("Failed to get relative path")?;
        let dst_path = dst.join(relative);

        if src_path.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(src_path, &dst_path)?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle --keygen flag (legacy, still supported)
    if args.keygen {
        let keypair = AgentKeyPair::generate()?;
        println!("Generated new master key pair:");
        println!("  SAVANT_MASTER_SECRET_KEY={}", keypair.secret_key);
        println!("  SAVANT_MASTER_PUBLIC_KEY={}", keypair.public_key);
        println!("  SAVANT_MASTER_KEY_ID={}", keypair.key_id);
        return Ok(());
    }

    // Initialize tracing for all subcommands
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    match args.command {
        Some(Commands::TestSkill {
            skill_path,
            input,
            timeout,
        }) => cmd_test_skill(&skill_path, &input, timeout).await,
        Some(Commands::Backup {
            output,
            include_memory,
        }) => cmd_backup(&args.config, &output, include_memory).await,
        Some(Commands::Restore { input }) => cmd_restore(&args.config, &input).await,
        Some(Commands::ListAgents) => cmd_list_agents(&args.config).await,
        Some(Commands::Status) => cmd_status(&args.config).await,
        Some(Commands::Start) | None => cmd_start(args.config).await,
    }
}
