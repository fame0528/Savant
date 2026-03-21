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
    ///
    /// Routes to:
    /// - `WasmComponent` → WASM executor (wasmtime-based)
    /// - `LegacyNative` → Native executor (Landlock-sandboxed)
    /// - `DockerContainer` → Docker executor (bollard-based, full isolation)
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
            ExecutionMode::DockerContainer(image) => {
                match crate::docker::DockerToolExecutor::new(image.clone()) {
                    Ok(executor) => Box::new(executor),
                    Err(e) => {
                        tracing::error!(
                            "Failed to create Docker executor for image {}: {}",
                            image,
                            e
                        );
                        // Fall back to a no-op executor that returns the error
                        Box::new(FallbackExecutor {
                            error: format!("Docker executor init failed: {}", e),
                        })
                    }
                }
            }
            ExecutionMode::Reference => Box::new(FallbackExecutor {
                error: "ExecutionMode::Reference is documentation-only and cannot be executed"
                    .to_string(),
            }),
        }
    }
}

/// Fallback executor that returns an error when the primary executor fails to initialize.
struct FallbackExecutor {
    error: String,
}

#[async_trait]
impl ToolExecutor for FallbackExecutor {
    async fn execute(&self, _args: Value) -> Result<String, SavantError> {
        Err(SavantError::Unknown(self.error.clone()))
    }
}
