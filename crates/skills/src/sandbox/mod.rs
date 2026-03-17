use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::types::{CapabilityGrants, ExecutionMode};
use serde_json::Value;

pub mod native;
pub mod wasm;

/// Trait for executing a skill in a sandboxed environment.
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Executes the tool with the provided JSON arguments.
    /// Returns the standard output or an error describing the failure.
    async fn execute(&self, args: Value) -> Result<String, SavantError>;
}

/// Dispatches execution to the appropriate sandbox engine.
pub struct SandboxDispatcher;

impl SandboxDispatcher {
    /// Creates a boxed ToolExecutor based on the execution mode.
    pub fn create_executor(
        mode: &ExecutionMode,
        workspace_dir: std::path::PathBuf,
        capabilities: CapabilityGrants,
    ) -> Box<dyn ToolExecutor> {
        match mode {
            ExecutionMode::WasmComponent(url) => {
                Box::new(wasm::WassetteExecutor::new(url.clone(), workspace_dir))
            }
            ExecutionMode::LegacyNative(script) => Box::new(native::LegacyNativeExecutor::new(
                script.clone(),
                workspace_dir,
                capabilities,
            )),
        }
    }
}
