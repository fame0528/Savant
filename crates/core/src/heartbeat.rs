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
                let _ = tx.send(command);
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
}
