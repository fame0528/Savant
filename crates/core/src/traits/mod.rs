use std::pin::Pin;
use std::future::Future;
use crate::error::SavantError;
use crate::types::EventFrame;

/// Channel Adapter Trait
pub trait ChannelAdapter: Send + Sync {
    /// Retrieve the adapter name.
    fn name(&self) -> &str;

    /// Send an event frame (outbound).
    fn send_event(&self, event: EventFrame) -> Pin<Box<dyn Future<Output = Result<(), SavantError>> + Send>>;

    /// Handle an incoming event (inbound).
    fn handle_event(&self, event: EventFrame) -> Pin<Box<dyn Future<Output = Result<(), SavantError>> + Send>>;
}

/// Skill Executor Trait
pub trait SkillExecutor: Send + Sync {
    /// Execute a skill.
    fn execute(&self, payload: &str) -> Pin<Box<dyn Future<Output = Result<String, SavantError>> + Send>>;
}
