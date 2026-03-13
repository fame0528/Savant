//! ECHO Configuration Watcher
//!
//! Glues the pipeline together by listening for workspace changes, 
//! triggering compilation, and performing atomic hot-swaps.

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent};
use std::sync::Arc;
use tokio::sync::mpsc;
use std::time::Duration;
use std::path::{Path, PathBuf};
use tracing::{info, error};

use crate::registry::HotSwappableRegistry;
use crate::compiler::EchoCompiler;

/// Spawns the ECHO watcher pipeline.
pub async fn spawn_echo_watcher(
    workspace_path: PathBuf,
    registry: Arc<HotSwappableRegistry>,
    compiler: Arc<EchoCompiler>
) -> Result<(), savant_core::error::SavantError> {
    let (tx, mut rx) = mpsc::channel(100);

    // Run the blocking `notify` watcher in a dedicated thread
    let workspace_path_thread = workspace_path.clone();
    
    // We attempt to initialize the debouncer before spawning the thread to catch errors early
    let tx_clone = tx.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(500), 
        move |res: Result<Vec<DebouncedEvent>, _>| {
            if let Ok(events) = res {
                for event in events {
                    let _ = tx_clone.blocking_send(event.path);
                }
            }
        }
    ).map_err(|e| savant_core::error::SavantError::Unknown(format!("Failed to create ECHO debouncer: {}", e)))?;

    debouncer.watcher()
        .watch(Path::new(&workspace_path_thread), RecursiveMode::Recursive)
        .map_err(|e| savant_core::error::SavantError::Unknown(format!("ECHO failed to watch workspace: {}", e)))?;

    std::thread::spawn(move || {
        // Move ownership of debouncer into the thread to keep it alive
        let _keep_alive = debouncer;
        loop { std::thread::sleep(Duration::from_secs(3600)); }
    });

    // Async receiver loop handling the actual compilation and hot-swapping
    tokio::spawn(async move {
        while let Some(path) = rx.recv().await {
            // Check if the modified file is a "trigger" file (e.g., manifest.json)
            if let Some(filename) = path.file_name() {
                if filename == "manifest.json" {
                    info!("ECHO detected configuration update at {:?}. Initiating pipeline.", path);
                    
                    // Extract the tool name/directory from the path
                    // We assume project structure: workspace/tool_name/manifest.json
                    if let Some(parent) = path.parent() {
                        if let Some(tool_name) = parent.file_name().and_then(|n| n.to_str()) {
                            let tool_dir = tool_name; // Relative to workspace_root
                            
                            // 1. Compile the tool securely
                            match compiler.compile_to_wasm(tool_dir).await {
                                Ok(wasm_bytes) => {
                                    // 2. Perform Lock-Free Hot-Swap
                                    if let Err(e) = registry.hot_load_component(tool_name, wasm_bytes) {
                                        error!("Failed to hot-swap component '{}': {}", tool_name, e);
                                    }
                                },
                                Err(e) => error!("ECHO Compilation aborted for '{}': {}", tool_name, e),
                            }
                        }
                    }
                }
            }
        }
    });

    Ok(())
}
