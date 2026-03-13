//! Sandboxed WASM Compilation Pipeline
//!
//! Wraps `cargo build` in a strict jail (Landlock on Linux). Prevents the AI 
//! from accidentally (or maliciously) accessing host environment variables or 
//! reading sensitive files during the compilation phase.

use std::process::Stdio;
use std::path::{PathBuf};
use tokio::process::Command;
use thiserror::Error;
use tracing::{info, error};

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("Compilation failed: {0}")]
    BuildFailed(String),
    #[error("Sandbox error: {0}")]
    SandboxError(String),
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Environment error: {0}")]
    EnvError(String),
}

/// The ECHO Compiler handles sandboxed Rust-to-WASM builds.
pub struct EchoCompiler {
    workspace_root: PathBuf,
}

impl EchoCompiler {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Compiles a generated Rust project into a WASM component.
    pub async fn compile_to_wasm(&self, project_dir: &str) -> Result<Vec<u8>, CompilerError> {
        let full_project_path = self.workspace_root.join(project_dir);
        let output_wasm = full_project_path.join("target/wasm32-wasip2/release/echo_tool.wasm");

        info!("ECHO initiating sandboxed compilation for {:?}", full_project_path);

        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .arg("--target=wasm32-wasip2")
            .arg("--release")
            .current_dir(&full_project_path)
            .env_clear()
            // Provide minimal path for the compiler
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "linux")]
        {
            use landlock::{Ruleset, ABI, AccessFs, PathBeneath};
            let project_path_clone = full_project_path.clone();
            unsafe {
                cmd.pre_exec(move || {
                    let abi = ABI::V1;
                    let ruleset = Ruleset::new()
                        .handle_access(AccessFs::from_all(abi)).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Landlock ruleset init failed"))?
                        .create().map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Landlock creation failed"))?
                        .add_rule(PathBeneath::new(&project_path_clone, AccessFs::from_all(abi)).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Project rule failed"))?).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Rule add failed"))?
                        // Add common system paths if needed for compilation (simplified for now)
                        .add_rule(PathBeneath::new("/usr/lib", AccessFs::from_read(abi)).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "System lib rule failed"))?).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Rule add failed"))?;
                    
                    ruleset.restrict_self().map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Landlock restriction failed"))?;
                    Ok(())
                });
            }
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            info!("Warning: ECHO sandboxing (Landlock) is not supported on this OS. Running without sandbox.");
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("ECHO Compilation Failed:\n{}", stderr);
            return Err(CompilerError::BuildFailed(stderr.to_string()));
        }

        info!("Compilation successful. Output target generated.");

        let wasm_bytes = tokio::fs::read(&output_wasm).await?;
        Ok(wasm_bytes)
    }
}
