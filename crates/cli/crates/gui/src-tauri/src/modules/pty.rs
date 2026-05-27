//! PTY module — manages pseudo-terminal sessions via portable-pty

use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, NativePtySystem, PtyPair, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use tracing::{debug, error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyEvent {
    pub data: Option<String>,
    pub exited: Option<i32>,
    pub error: Option<String>,
}

pub struct PtyInstance {
    writer: Box<dyn Write + Send>,
    pair: PtyPair,
}

pub struct PtyService {
    instances: HashMap<String, PtyInstance>,
    system: NativePtySystem,
}

impl PtyService {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            system: NativePtySystem::default(),
        }
    }

    pub fn create(
        &mut self,
        id: &str,
        shell: Option<&str>,
        cwd: Option<&str>,
        app_handle: Option<tauri::AppHandle>,
    ) -> Result<()> {
        let pty_size = PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = self
            .system
            .openpty(pty_size)
            .context("Failed to open PTY")?;

        let shell = shell.unwrap_or(if cfg!(windows) {
            "powershell.exe"
        } else {
            "bash"
        });

        let mut cmd = CommandBuilder::new(shell);
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }

        let _child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn shell command")?;

        info!("PTY {} created with shell={} cwd={:?}", id, shell, cwd);

        // Spawn reader task with event emission
        let reader = pair
            .master
            .try_clone_reader()
            .context("Failed to clone PTY reader")?;
        let id_clone = id.to_string();
        let app_clone = app_handle.clone();

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut reader = reader;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // PTY closed — emit exit event
                        crate::emit_pty_event(
                            &app_clone,
                            &id_clone,
                            PtyEvent {
                                data: None,
                                exited: Some(0),
                                error: None,
                            },
                        );
                        break;
                    }
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buf[..n]).to_string();
                        debug!("PTY {} output ({} bytes)", id_clone, n);
                        // Emit output event to frontend
                        crate::emit_pty_event(
                            &app_clone,
                            &id_clone,
                            PtyEvent {
                                data: Some(text),
                                exited: None,
                                error: None,
                            },
                        );
                    }
                    Err(e) => {
                        error!("PTY {} read error: {}", id_clone, e);
                        // Emit error event to frontend
                        crate::emit_pty_event(
                            &app_clone,
                            &id_clone,
                            PtyEvent {
                                data: None,
                                exited: None,
                                error: Some(e.to_string()),
                            },
                        );
                        break;
                    }
                }
            }
            info!("PTY {} reader closed", id_clone);
        });

        let writer = pair
            .master
            .take_writer()
            .context("Failed to take PTY writer")?;

        self.instances
            .insert(id.to_string(), PtyInstance { writer, pair });

        Ok(())
    }

    pub fn write(&mut self, id: &str, data: &str) -> Result<()> {
        let instance = self
            .instances
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("PTY {} not found", id))?;
        instance
            .writer
            .write_all(data.as_bytes())
            .context("Failed to write to PTY")?;
        instance.writer.flush().context("Failed to flush PTY")?;
        Ok(())
    }

    pub fn resize(&mut self, id: &str, cols: u16, rows: u16) -> Result<()> {
        let instance = self
            .instances
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("PTY {} not found", id))?;
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        instance
            .pair
            .master
            .resize(size)
            .context("Failed to resize PTY")?;
        Ok(())
    }

    pub fn close(&mut self, id: &str) -> Result<()> {
        self.instances.remove(id);
        info!("PTY {} closed", id);
        Ok(())
    }

    pub fn list(&self) -> Vec<String> {
        self.instances.keys().cloned().collect()
    }
}
