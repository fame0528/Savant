//! Event router for gateway events
//!
//! Routes incoming events to registered handlers by event type.

use savant_core::types::EventFrame;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

type EventHandler = Box<dyn Fn(EventFrame) + Send + Sync>;

/// Routes events to registered handlers
#[derive(Clone)]
pub struct EventRouter {
    handlers: Arc<RwLock<HashMap<String, Vec<EventHandler>>>>,
}

impl EventRouter {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe to events of a specific type
    pub async fn subscribe<F>(&self, event_type: String, handler: F)
    where
        F: Fn(EventFrame) + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers
            .entry(event_type)
            .or_default()
            .push(Box::new(handler));
    }

    /// Route an event to all registered handlers
    pub async fn route(&self, event: EventFrame) {
        let handlers = self.handlers.read().await;

        // Route to specific event type handlers
        if let Some(type_handlers) = handlers.get(&event.event_type) {
            for handler in type_handlers {
                handler(event.clone());
            }
        }

        // Route to wildcard handlers (catch-all)
        if let Some(wildcard_handlers) = handlers.get("*") {
            for handler in wildcard_handlers {
                handler(event.clone());
            }
        }

        debug!("Routed event: {}", event.event_type);
    }

    /// Subscribe to all events (wildcard)
    pub async fn subscribe_all<F>(&self, handler: F)
    where
        F: Fn(EventFrame) + Send + Sync + 'static,
    {
        self.subscribe("*".to_string(), handler).await;
    }
}

impl Default for EventRouter {
    fn default() -> Self {
        Self::new()
    }
}
