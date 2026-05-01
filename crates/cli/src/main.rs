#![allow(clippy::disallowed_methods)]
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use savant_agent::orchestration::ignition::IgnitionService;
use savant_core::config::Config;
use savant_core::crypto::AgentKeyPair;
use std::path::PathBuf;

/// Tracing layer that publishes log messages to the global debug log channel
/// for real-time streaming to the dashboard via WebSocket.
struct DebugLogLayer;

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for DebugLogLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = LogVisitor::default();
        event.record(&mut visitor);
        let meta = event.metadata();
        let msg = format!("[{}] {}", meta.level(), visitor.message);
        // Silently ignore send errors — no receivers is expected before gateway connects.
        // Do NOT log here: it would trigger this layer recursively.
        let _ = savant_core::bus::debug_log_sender().send(msg);
    }
}

#[derive(Default)]
struct LogVisitor {
    message: String,
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
}

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

    /// Heartbeat and Cognitive Diary diagnostics
    Heartbeat {
        /// Manually trigger a proactive pulse
        #[arg(long)]
        pulse: bool,

        /// Force a specific cognitive lens ID
        #[arg(short, long)]
        lens: Option<String>,

        /// Check the diversity distribution of recent logs
        #[arg(long)]
        check: bool,
    },

    /// Inspect system state (WorkingBuffer, offsets, etc.)
    State {
        /// Inspect persistent agent state
        #[arg(long)]
        inspect: bool,
    },
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
    print_phase(1, "Loading configuration");
    print_phase(2, "Igniting substrate");

    let _ignition = IgnitionService::ignite(config_path.as_deref()).await?;

    print_phase(3, "Substrate active");

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
        eprintln!(
            "{} Skill file not found: {}",
            "Error:".red().bold(),
            skill_path
        );
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
    let input_value: serde_json::Value =
        serde_json::from_str(input).unwrap_or_else(|_| serde_json::json!({"raw": input}));

    println!(
        "{} Input: {}",
        "→".bright_black(),
        serde_json::to_string_pretty(&input_value).unwrap_or_default()
    );
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
                println!(
                    "{}",
                    format!("❌ Skill test TIMED OUT after {}s", timeout_secs)
                        .red()
                        .bold()
                );
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

async fn cmd_backup(
    config_path: &Option<String>,
    output: &str,
    include_memory: bool,
) -> Result<()> {
    println!("{}", "=== Savant Database Backup ===".cyan().bold());
    println!();

    let config = Config::load_from(config_path.as_deref()).unwrap_or_else(|_| Config::default());

    let db_path = PathBuf::from(&config.system.db_path);
    let output_path = PathBuf::from(output);

    println!("{} Database: {:?}", "→".bright_black(), db_path);
    println!("{} Output:   {:?}", "→".bright_black(), output_path);

    // Create the backup by copying the database directory
    if !db_path.exists() {
        eprintln!(
            "{} Database path does not exist: {:?}",
            "Error:".red().bold(),
            db_path
        );
        std::process::exit(1);
    }

    // Create backup archive
    println!("{} Creating backup...", "→".bright_black());

    // Copy all files from the database directory to the output
    if output_path.exists() {
        std::fs::remove_dir_all(&output_path)
            .context("Failed to clean existing backup directory")?;
    }

    copy_dir_recursive(&db_path, &output_path).context("Failed to copy database files")?;

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
        eprintln!(
            "{} Backup path does not exist: {:?}",
            "Error:".red().bold(),
            input_path
        );
        std::process::exit(1);
    }

    let config = Config::load_from(config_path.as_deref()).unwrap_or_else(|_| Config::default());

    let db_path = PathBuf::from(&config.system.db_path);

    println!("{} Backup:   {:?}", "→".bright_black(), input_path);
    println!("{} Target:   {:?}", "→".bright_black(), db_path);

    // Backup existing database before restore
    if db_path.exists() {
        let pre_restore_backup = db_path.with_extension("pre-restore");
        println!(
            "{} Backing up existing database to {:?}...",
            "→".bright_black(),
            pre_restore_backup
        );
        copy_dir_recursive(&db_path, &pre_restore_backup)
            .context("Failed to backup existing database")?;
    }

    // Restore from backup
    println!("{} Restoring database...", "→".bright_black());

    if db_path.exists() {
        std::fs::remove_dir_all(&db_path).context("Failed to clean existing database")?;
    }

    copy_dir_recursive(&input_path, &db_path).context("Failed to restore database files")?;

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

    let config = Config::load_from(config_path.as_deref()).unwrap_or_else(|_| Config::default());

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
        println!(
            "{} Workspaces: {} ({agent_count} agents)",
            "✓".green(),
            "Present".green()
        );
    } else {
        println!("{} Workspaces: {}", "⚠".yellow(), "Not found".yellow());
    }

    // Check skills
    let skills = PathBuf::from("./skills");
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
        let relative = src_path
            .strip_prefix(src)
            .context("Failed to get relative path")?;
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
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    if let Err(e) = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(filter))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_target(false)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false),
        )
        .with(DebugLogLayer)
        .try_init()
    {
        tracing::warn!("[cli] Failed to initialize tracing subscriber: {}", e);
    }

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
        Some(Commands::Heartbeat { pulse, lens, check }) => {
            cmd_heartbeat(&args.config, pulse, lens, check).await
        }
        Some(Commands::State { inspect }) => cmd_state(&args.config, inspect).await,
        Some(Commands::Start) | None => cmd_start(args.config).await,
    }
}

async fn cmd_heartbeat(
    config_path: &Option<String>,
    pulse: bool,
    lens: Option<String>,
    check: bool,
) -> Result<()> {
    println!("{}", "=== Savant Heartbeat Diagnostics ===".cyan().bold());
    let _config = Config::load_from(config_path.as_deref()).unwrap_or_else(|_| Config::default());

    if pulse {
        println!("{} Connecting to Nexus bridge...", "→".bright_black());
        let nexus = savant_core::bus::NexusBridge::new();

        println!("{} Triggering manual pulse...", "→".bright_black());
        if let Some(ref l) = lens {
            println!("{} Forced Lens: {}", "→".bright_black(), l.green());
        }

        let payload = serde_json::json!({ "lens": lens });
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
            let content = std::fs::read_to_string(md_path)?;
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

async fn cmd_state(config_path: &Option<String>, inspect: bool) -> Result<()> {
    println!("{}", "=== Savant State Inspector ===".cyan().bold());
    let config = Config::load_from(config_path.as_deref()).unwrap_or_else(|_| Config::default());

    if inspect {
        println!("{} Reading Sovereign State (WAL)...", "→".bright_black());
        let substrate_path = std::path::PathBuf::from(&config.system.substrate_path);
        let _proactive = config.swarm.heartbeat_interval > 0; // Simplified check

        let state_file = substrate_path.join("DEV-SESSION-STATE.md");
        if state_file.exists() {
            println!(
                "  {} File: {:?}",
                "•".cyan(),
                state_file.file_name().unwrap_or_default()
            );
            let content = std::fs::read_to_string(state_file)?;
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
                context_file.file_name().unwrap()
            );
            let content = std::fs::read_to_string(context_file)?;
            println!("{}", content.bright_black());
        }
    }

    Ok(())
}
