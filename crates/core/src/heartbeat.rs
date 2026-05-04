use crate::error::SavantError;
use crate::types::HeartbeatTask;
use tokio::sync::broadcast;
use tokio_cron_scheduler::{Job, JobScheduler};

/// Heartbeat Scheduler for managing cron-like tasks.
pub struct HeartbeatScheduler {
    scheduler: JobScheduler,
    event_tx: broadcast::Sender<String>,
}

impl HeartbeatScheduler {
    /// Initializes a new scheduler and a broadcast channel for events.
    pub async fn new() -> Result<Self, SavantError> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| SavantError::Unknown(format!("Scheduler init error: {}", e)))?;
        let (event_tx, _) = broadcast::channel(100);
        let _watchdog = crate::pulse::watchdog::SovereignWatchdog::new();

        Ok(Self {
            scheduler,
            event_tx,
        })
    }

    /// Adds a task to the scheduler.
    pub async fn add_task(&self, task: HeartbeatTask) -> Result<(), SavantError> {
        let tx = self.event_tx.clone();
        let task_id = task.id.clone();
        let command = task.command.clone();

        let job = Job::new_async(task.schedule.as_str(), move |_uuid, _l| {
            let tx = tx.clone();
            let command = command.clone();
            let task_id = task_id.clone();

            Box::pin(async move {
                tracing::info!("Triggered heartbeat job: {}", task_id);
                if let Err(e) = tx.send(command) {
                    tracing::warn!(
                        "[core::heartbeat] Failed to send heartbeat command: {:?}",
                        e
                    );
                }
            })
        })
        .map_err(|e| SavantError::Unknown(format!("Job creation error: {}", e)))?;

        self.scheduler
            .add(job)
            .await
            .map_err(|e| SavantError::Unknown(format!("Scheduler add error: {}", e)))?;

        Ok(())
    }

    /// Starts the scheduler.
    pub async fn start(&self) -> Result<(), SavantError> {
        self.scheduler
            .start()
            .await
            .map_err(|e| SavantError::Unknown(format!("Scheduler start error: {}", e)))?;
        Ok(())
    }

    /// Returns a receiver for heartbeat events.
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.event_tx.subscribe()
    }

    /// Registers the EvolvePulse cron job (runs every N hours for self-reflection).
    pub async fn register_evolve_pulse(&self) -> Result<(), SavantError> {
        self.add_task(HeartbeatTask {
            id: "evolve_pulse".to_string(),
            schedule: "0 */4 * * * *".to_string(),
            command: "EVOLVE_PULSE".to_string(),
            last_run: None,
            next_run: None,
        })
        .await
    }

    pub async fn register_weekly_digest(&self) -> Result<(), SavantError> {
        self.add_task(HeartbeatTask {
            id: "weekly_evolution_digest".to_string(),
            schedule: "0 0 9 * * Mon *".to_string(),
            command: "WEEKLY_EVOLUTION_DIGEST".to_string(),
            last_run: None,
            next_run: None,
        })
        .await
    }
}
