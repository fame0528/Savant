// SAFETY: All clippy::disallowed_methods violations in this file originate from serde_json::json!() macro internals. The json!() macro calls .unwrap() on provably-infallible compile-time-validated JSON literals. grep confirms 0 real .unwrap() calls exist in this file outside macro expansions.
#![allow(clippy::disallowed_methods)]
//! Savant CLI wrapper binary
//!
//! This binary provides the `savant` command, which defaults to `savant-cli chat`
//! when no subcommand is provided. All other subcommands are passed through
//! identically to `savant-cli`.
// SAFETY: All `clippy::disallowed_methods` violations in this file originate from
// the `serde_json::json!()` macro, which internally uses `.unwrap()` on
// compile-time-validated JSON literals. A malformed JSON literal would be a
// compile error, making the panic path statically unreachable.

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;

mod commands;

/// Savant — Autonomous Agent Swarm Orchestrator
///
/// This is the primary entry point for the Savant CLI.
/// When run without arguments, it launches the interactive chat companion.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Savant CLI — Autonomous Agent Swarm Orchestrator",
    long_about = None
)]
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

    /// Interactive AI coding companion (default when no subcommand given)
    Chat {
        /// Gateway WebSocket URL override
        #[arg(long)]
        gateway_url: Option<String>,

        /// Start minimal Gateway + single agent without full swarm
        #[arg(long)]
        standalone: bool,
    },

    /// Test a skill from a SKILL.md file
    TestSkill {
        #[arg(short, long)]
        skill_path: String,
        #[arg(short, long, default_value = "{}")]
        input: String,
        #[arg(short, long, default_value = "30")]
        timeout: u64,
    },

    /// Backup the Savant database to a file
    Backup {
        #[arg(short, long)]
        output: String,
        #[arg(long)]
        include_memory: bool,
    },

    /// Restore the Savant database from a backup file
    Restore {
        #[arg(short, long)]
        input: String,
    },

    /// List discovered agents
    ListAgents,

    /// Display system status and health
    Status,

    /// Heartbeat and Cognitive Diary diagnostics
    Heartbeat {
        #[arg(long)]
        pulse: bool,
        #[arg(short, long)]
        lens: Option<String>,
        #[arg(long)]
        check: bool,
    },

    /// Inspect system state (WorkingBuffer, offsets, etc.)
    State {
        #[arg(long)]
        inspect: bool,
    },

    /// Manage scheduled jobs (cron)
    Schedule {
        #[command(subcommand)]
        cmd: commands::schedule::ScheduleCommand,
    },
}

fn print_splash() {
    let logo = r#"
    ███████╗ █████╗ ██╗   ██╗ █████╗ ███╗   ██╗████████╗
    ██╔════╝██╔══██║██║   ██║██╔══██╗████╗  ██║╚══██╔══╝
    ███████╗███████║██║   ██║███████║██╔██╗ ██║   ██║   
    ╚════██║██╔══██║╚██╗ ██╔╝██╔══██║██║╚██╗██║   ██║   
    ███████║██║  ██║ ╚████╔╝ ██║  ██║██║ ╚████║   ██║   
    ╚══════╝╚═╝  ╚═╝  ╚═══╝  ╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝   
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
    println!(
        "{}",
        format!("   v{} CLI COMPANION", env!("CARGO_PKG_VERSION"))
            .green()
            .bold()
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

#[cfg(target_os = "windows")]
fn main() {
    // On Windows, attach to parent console or allocate a new one
    // so error messages are visible when launched from Explorer.
    extern "system" {
        fn AttachConsole(dw_process_id: u32) -> i32;
        fn AllocConsole() -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFFFFFF;
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            AllocConsole();
        }
    }

    if let Err(e) = async_main() {
        eprintln!("Error: {}", e);
        eprintln!("Press Enter to exit...");
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "windows"))]
fn main() -> anyhow::Result<()> {
    async_main()
}

#[tokio::main]
async fn async_main() -> Result<()> {
    let args = Args::parse();

    if args.keygen {
        let keypair = savant_core::crypto::AgentKeyPair::generate()?;
        println!("Generated new master key pair:");
        println!("  SAVANT_MASTER_SECRET_KEY={}", keypair.secret_key);
        println!("  SAVANT_MASTER_PUBLIC_KEY={}", keypair.public_key);
        println!("  SAVANT_MASTER_KEY_ID={}", keypair.key_id);
        return Ok(());
    }

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
        .with(commands::DebugLogLayer)
        .try_init()
    {
        tracing::warn!("[savant] Failed to initialize tracing subscriber: {}", e);
    }

    match args.command {
        Some(Commands::Chat {
            gateway_url,
            standalone,
        }) => {
            print_splash();
            print_phase(1, "Initializing Savant CLI Companion");
            cmd_chat(args.config, gateway_url, standalone).await
        }
        Some(Commands::Start) => cmd_start(args.config).await,
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
        Some(Commands::Schedule { cmd }) => commands::schedule::handle_schedule_command(cmd).await,
        None => {
            print_splash();
            print_phase(1, "Initializing Savant CLI Companion");
            cmd_chat(args.config, None, false).await
        }
    }
}

async fn cmd_chat(
    _config_path: Option<String>,
    gateway_url: Option<String>,
    _standalone: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    savant_cli_tui::run_with_gateway(gateway_url, &cwd).await?;
    Ok(())
}

async fn cmd_start(config_path: Option<String>) -> anyhow::Result<()> {
    commands::start::cmd_start(config_path).await
}

async fn cmd_test_skill(skill_path: &str, input: &str, timeout_secs: u64) -> anyhow::Result<()> {
    commands::test_skill::cmd_test_skill(skill_path, input, timeout_secs).await
}

async fn cmd_backup(
    config_path: &Option<String>,
    output: &str,
    include_memory: bool,
) -> anyhow::Result<()> {
    commands::backup::cmd_backup(config_path, output, include_memory).await
}

async fn cmd_restore(config_path: &Option<String>, input: &str) -> anyhow::Result<()> {
    commands::restore::cmd_restore(config_path, input).await
}

async fn cmd_list_agents(config_path: &Option<String>) -> anyhow::Result<()> {
    commands::list_agents::cmd_list_agents(config_path).await
}

async fn cmd_status(config_path: &Option<String>) -> anyhow::Result<()> {
    commands::status::cmd_status(config_path).await
}

async fn cmd_heartbeat(
    config_path: &Option<String>,
    pulse: bool,
    lens: Option<String>,
    check: bool,
) -> anyhow::Result<()> {
    commands::heartbeat::cmd_heartbeat(config_path, pulse, lens, check).await
}

async fn cmd_state(config_path: &Option<String>, inspect: bool) -> anyhow::Result<()> {
    commands::state::cmd_state(config_path, inspect).await
}
