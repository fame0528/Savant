//! Schedule management CLI commands.

use clap::Subcommand;
use savant_core::heartbeat::{
    HeartbeatScheduler, MissedExecutionPolicy, ScheduleConfig, SchedulePayload,
};

#[derive(Subcommand, Debug)]
pub enum ScheduleCommand {
    /// List all schedules
    List,
    /// Add a new schedule
    Add {
        /// Schedule name
        #[arg(long)]
        name: String,
        /// Cron expression (e.g., "0 9 * * *")
        #[arg(long)]
        cron: String,
        /// Prompt for agent turn
        #[arg(long)]
        prompt: Option<String>,
        /// Skills to attach (comma-separated)
        #[arg(long)]
        skills: Option<String>,
        /// System event to emit
        #[arg(long)]
        event: Option<String>,
    },
    /// Remove a schedule by ID
    Remove {
        /// Schedule ID
        id: String,
    },
    /// Show schedule statistics
    Stats,
}

#[allow(dead_code)] // Called from savant.rs binary
pub async fn handle_schedule_command(cmd: ScheduleCommand) -> anyhow::Result<()> {
    let scheduler = HeartbeatScheduler::new().await?;

    match cmd {
        ScheduleCommand::List => {
            let schedules = scheduler.list_schedules()?;
            if schedules.is_empty() {
                println!("No schedules configured.");
                return Ok(());
            }
            println!(
                "{:<36} {:<25} {:<15} {:<8} {:<10}",
                "ID", "NAME", "CRON", "ENABLED", "TYPE"
            );
            println!("{}", "-".repeat(100));
            for s in &schedules {
                let payload_type = match &s.payload {
                    SchedulePayload::PulseTrigger => "pulse",
                    SchedulePayload::AgentTurn { .. } => "agent_turn",
                    SchedulePayload::SystemEvent { .. } => "system_event",
                };
                println!(
                    "{:<36} {:<25} {:<15} {:<8} {:<10}",
                    s.id, s.name, s.cron_expr, s.enabled, payload_type
                );
            }
            println!("\nTotal: {} schedules", schedules.len());
        }

        ScheduleCommand::Add {
            name,
            cron,
            prompt,
            skills,
            event,
        } => {
            let payload = if let Some(prompt) = prompt {
                let skills_list = skills
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();
                SchedulePayload::AgentTurn {
                    prompt,
                    skills: skills_list,
                }
            } else if let Some(event) = event {
                SchedulePayload::SystemEvent { event }
            } else {
                SchedulePayload::PulseTrigger
            };

            let id = uuid::Uuid::new_v4().to_string();
            let config = ScheduleConfig {
                id: id.clone(),
                name,
                cron_expr: cron,
                timezone: None,
                payload,
                enabled: true,
                missed_policy: MissedExecutionPolicy::default(),
                last_run_at: None,
                next_run_at: None,
                consecutive_errors: 0,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
            };

            scheduler.save_schedule(&config)?;
            println!("Schedule created: {}", id);
        }

        ScheduleCommand::Remove { id } => match scheduler.remove_schedule(&id)? {
            true => println!("Schedule {} removed.", id),
            false => println!("Schedule {} not found.", id),
        },

        ScheduleCommand::Stats => {
            let schedules = scheduler.list_schedules()?;
            let total = schedules.len();
            let enabled = schedules.iter().filter(|s| s.enabled).count();
            let disabled = total - enabled;
            let with_errors = schedules
                .iter()
                .filter(|s| s.consecutive_errors > 0)
                .count();
            println!("Schedule Statistics:");
            println!("  Total:    {}", total);
            println!("  Enabled:  {}", enabled);
            println!("  Disabled: {}", disabled);
            println!("  With errors: {}", with_errors);
        }
    }

    Ok(())
}
