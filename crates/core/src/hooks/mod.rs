//! Hook/Lifecycle System — runtime extensibility for agent lifecycle events.
//!
//! 6 hook events with 3 execution strategies:
//! - **Void** (fire-and-forget, parallel): All handlers run concurrently
//! - **Modifying** (sequential, priority-ordered): Handlers can modify the payload
//! - **Claiming** (first-wins): First handler claiming success stops iteration

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Hook event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookEvent {
    BeforeToolCall,
    AfterToolCall,
    LlmInput,
    LlmOutput,
    SessionStart,
    SessionEnd,
}

/// Hook execution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookStrategy {
    /// All handlers run in parallel, errors logged
    Void,
    /// Handlers run sequentially by priority, first can modify
    Modifying,
    /// First handler claiming success wins
    Claiming,
}

/// Hook priority — higher runs first.
pub type HookPriority = i32;

/// Hook handler trait for void hooks.
#[async_trait::async_trait]
pub trait VoidHookHandler: Send + Sync {
    fn event(&self) -> HookEvent;
    fn priority(&self) -> HookPriority {
        0
    }
    async fn handle(&self, context: &HookContext);
}

/// Hook context passed to handlers.
#[derive(Debug, Clone)]
pub struct HookContext {
    pub event: HookEvent,
    pub tool_name: Option<String>,
    pub content: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Hook result for modifying hooks.
#[derive(Debug, Clone)]
pub enum HookResult {
    /// Content was modified
    Modified(String),
    /// Content unchanged
    Unchanged,
}

/// Modifying hook handler trait.
#[async_trait::async_trait]
pub trait ModifyingHookHandler: Send + Sync {
    fn event(&self) -> HookEvent;
    fn priority(&self) -> HookPriority {
        0
    }
    async fn handle(&self, context: HookContext) -> HookResult;
}

/// Hook registration entry.
struct HookRegistration {
    handler: Arc<dyn VoidHookHandler>,
    priority: HookPriority,
}

/// Hook registry — manages lifecycle hooks.
pub struct HookRegistry {
    handlers: RwLock<HashMap<HookEvent, Vec<HookRegistration>>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a void hook handler.
    pub async fn register_void(&self, handler: impl VoidHookHandler + 'static) {
        let event = handler.event();
        let priority = handler.priority();
        let mut handlers = self.handlers.write().await;
        let entry = handlers.entry(event).or_default();
        entry.push(HookRegistration {
            handler: Arc::new(handler),
            priority,
        });
        // Sort by priority descending (highest first)
        entry.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Runs void hooks for an event (parallel via tokio::spawn).
    pub async fn run_void(&self, context: &HookContext) {
        let handlers = self.handlers.read().await;
        if let Some(event_handlers) = handlers.get(&context.event) {
            let mut tasks = Vec::new();
            for reg in event_handlers {
                let ctx = context.clone();
                let handler = reg.handler.clone();
                tasks.push(tokio::spawn(async move {
                    handler.handle(&ctx).await;
                }));
            }
            for task in tasks {
                let _ = task.await;
            }
        }
    }

    /// Gets the number of registered handlers for an event.
    pub async fn handler_count(&self, event: HookEvent) -> usize {
        let handlers = self.handlers.read().await;
        handlers.get(&event).map(|v| v.len()).unwrap_or(0)
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Built-in hook implementations
// ============================================================================

/// Logging hook — logs tool calls and outputs.
pub struct ToolCallLogger;

#[async_trait::async_trait]
impl VoidHookHandler for ToolCallLogger {
    fn event(&self) -> HookEvent {
        HookEvent::AfterToolCall
    }
    fn priority(&self) -> HookPriority {
        0
    }

    async fn handle(&self, context: &HookContext) {
        if let Some(ref tool_name) = context.tool_name {
            tracing::debug!("[HOOK] Tool executed: {}", tool_name);
        }
    }
}

/// LLM input logger — logs context before LLM call.
pub struct LlmInputLogger;

#[async_trait::async_trait]
impl VoidHookHandler for LlmInputLogger {
    fn event(&self) -> HookEvent {
        HookEvent::LlmInput
    }
    fn priority(&self) -> HookPriority {
        0
    }

    async fn handle(&self, context: &HookContext) {
        if let Some(ref content) = context.content {
            tracing::debug!("[HOOK] LLM input: {} chars", content.len());
        }
    }
}

/// LLM output logger — logs response after LLM call.
pub struct LlmOutputLogger;

#[async_trait::async_trait]
impl VoidHookHandler for LlmOutputLogger {
    fn event(&self) -> HookEvent {
        HookEvent::LlmOutput
    }
    fn priority(&self) -> HookPriority {
        0
    }

    async fn handle(&self, context: &HookContext) {
        if let Some(ref content) = context.content {
            tracing::debug!("[HOOK] LLM output: {} chars", content.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler;

    #[async_trait::async_trait]
    impl VoidHookHandler for TestHandler {
        fn event(&self) -> HookEvent {
            HookEvent::BeforeToolCall
        }
        async fn handle(&self, _context: &HookContext) {}
    }

    #[tokio::test]
    async fn test_hook_registry_register_and_count() {
        let registry = HookRegistry::new();
        registry.register_void(TestHandler).await;
        assert_eq!(registry.handler_count(HookEvent::BeforeToolCall).await, 1);
        assert_eq!(registry.handler_count(HookEvent::AfterToolCall).await, 0);
    }

    #[tokio::test]
    async fn test_hook_registry_run_void() {
        let registry = HookRegistry::new();
        registry.register_void(ToolCallLogger).await;
        let context = HookContext {
            event: HookEvent::AfterToolCall,
            tool_name: Some("shell".to_string()),
            content: None,
            metadata: HashMap::new(),
        };
        registry.run_void(&context).await;
    }
}
