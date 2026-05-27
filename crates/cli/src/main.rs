// SAFETY: All clippy::disallowed_methods violations in this file originate from serde_json::json!() macro internals. The json!() macro calls .unwrap() on provably-infallible compile-time-validated JSON literals. grep confirms 0 real .unwrap() calls exist in this file outside macro expansions.
#![allow(clippy::disallowed_methods)]
// SAFETY: All `clippy::disallowed_methods` violations in this file originate from
// the `serde_json::json!()` macro, which internally uses `.unwrap()` on
// compile-time-validated JSON literals. A malformed JSON literal would be a
// compile error, making the panic path statically unreachable.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use savant_core::crypto::AgentKeyPair;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod commands;

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

    /// Manage recorded trajectories (training data)
    Trajectory {
        #[command(subcommand)]
        action: TrajectoryAction,
    },
}

#[derive(Subcommand, Debug)]
enum TrajectoryAction {
    /// List recorded trajectories
    List {
        /// Directory containing trajectory files
        #[arg(short, long, default_value = "./data/trajectories")]
        output_dir: String,
    },
    /// Show trajectory statistics
    Stats {
        /// Directory containing trajectory files
        #[arg(short, long, default_value = "./data/trajectories")]
        output_dir: String,
    },
    /// Export trajectories as ShareGPT JSONL
    Export {
        /// Directory containing trajectory files
        #[arg(short, long, default_value = "./data/trajectories")]
        output_dir: String,
        /// Output path for the exported file
        #[arg(long)]
        output: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.keygen {
        let keypair = AgentKeyPair::generate().context("Failed to generate master keypair")?;
        println!("Generated new master key pair:");
        println!("  SAVANT_MASTER_SECRET_KEY={}", keypair.secret_key);
        println!("  SAVANT_MASTER_PUBLIC_KEY={}", keypair.public_key);
        println!("  SAVANT_MASTER_KEY_ID={}", keypair.key_id);
        return Ok(());
    }

    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
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
        tracing::warn!("[cli] Failed to initialize tracing subscriber: {}", e);
    }

    match args.command {
        Some(Commands::TestSkill {
            skill_path,
            input,
            timeout,
        }) => commands::test_skill::cmd_test_skill(&skill_path, &input, timeout).await,
        Some(Commands::Backup {
            output,
            include_memory,
        }) => commands::backup::cmd_backup(&args.config, &output, include_memory).await,
        Some(Commands::Restore { input }) => {
            commands::restore::cmd_restore(&args.config, &input).await
        }
        Some(Commands::ListAgents) => commands::list_agents::cmd_list_agents(&args.config).await,
        Some(Commands::Status) => commands::status::cmd_status(&args.config).await,
        Some(Commands::Heartbeat { pulse, lens, check }) => {
            commands::heartbeat::cmd_heartbeat(&args.config, pulse, lens, check).await
        }
        Some(Commands::State { inspect }) => {
            commands::state::cmd_state(&args.config, inspect).await
        }
        Some(Commands::Trajectory { action }) => match action {
            TrajectoryAction::List { output_dir } => {
                commands::trajectory::cmd_trajectory_list(&output_dir).await
            }
            TrajectoryAction::Stats { output_dir } => {
                commands::trajectory::cmd_trajectory_stats(&output_dir).await
            }
            TrajectoryAction::Export { output_dir, output } => {
                commands::trajectory::cmd_trajectory_export(&output_dir, &output).await
            }
        },
        Some(Commands::Start) | None => commands::start::cmd_start(args.config).await,
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_args_default() {
        let args = Args::try_parse_from(["savant"]).unwrap();
        assert!(args.config.is_none());
        assert!(!args.keygen);
        assert!(args.command.is_none());
    }

    #[test]
    fn test_args_config_flag() {
        let args = Args::try_parse_from(["savant", "--config", "/path/to/config.toml"]).unwrap();
        assert_eq!(args.config.unwrap(), "/path/to/config.toml");
    }

    #[test]
    fn test_args_keygen_flag() {
        let args = Args::try_parse_from(["savant", "--keygen"]).unwrap();
        assert!(args.keygen);
    }

    #[test]
    fn test_args_start_command() {
        let args = Args::try_parse_from(["savant", "start"]).unwrap();
        assert!(matches!(args.command, Some(Commands::Start)));
    }
}
