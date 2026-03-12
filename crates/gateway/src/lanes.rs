use savant_core::types::{RequestFrame, ResponseFrame};
use tokio::sync::{mpsc, Semaphore};
use std::sync::Arc;

/// A session lane for queuing tasks and messages per session.
pub struct SessionLane {
    pub tx: mpsc::Sender<RequestFrame>,
    pub response_tx: mpsc::Sender<ResponseFrame>,
}

impl SessionLane {
    /// Creates a new SessionLane with specified capacity and concurrency limits.
    #[must_use]
    pub fn new(capacity: usize, max_concurrent: usize) -> (Self, mpsc::Receiver<RequestFrame>, mpsc::Receiver<ResponseFrame>, Arc<Semaphore>) {
        let (tx, rx) = mpsc::channel(capacity);
        let (res_tx, res_rx) = mpsc::channel(capacity);
        (
            Self { tx, response_tx: res_tx },
            rx,
            res_rx,
            Arc::new(Semaphore::new(max_concurrent))
        )
    }

    /// Spawns a consumer task that processes messages from the lane.
    pub fn spawn_consumer(
        mut rx: mpsc::Receiver<RequestFrame>, 
        response_tx: mpsc::Sender<ResponseFrame>, 
        concurrency_limit: Arc<Semaphore>,
        nexus: Arc<savant_core::bus::NexusBridge>,
    ) {
        tokio::spawn(async move {
            while let Some(frame) = rx.recv().await {
                let _permit = concurrency_limit.acquire().await.unwrap();
                tracing::debug!("Processing frame for session: {}", frame.session_id.0);
                
                // 1. Process Global Directives
                if frame.payload.starts_with("DIRECTIVE:") {
                    let directive = frame.payload.trim_start_matches("DIRECTIVE:").to_string();
                    nexus.update_state("GLOBAL_DIRECTIVE".to_string(), directive).await;
                    
                    let response = ResponseFrame {
                        request_id: "ack".to_string(),
                        payload: "Global directive broadcasted to swarm.".to_string(),
                    };
                    let _ = response_tx.send(response).await;
                    continue;
                }

                // Placeholder: standard echo logic
                let response = ResponseFrame {
                    request_id: "ack".to_string(),
                    payload: format!("Accepted: {}", frame.payload),
                };
                let _ = response_tx.send(response).await;
            }
        });
    }
}
