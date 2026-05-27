//! Shell module — persistent shell sessions and background processes

use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use tracing::{debug, info, warn};

/// Lock a mutex, recovering from poison by taking the inner value anyway.
/// Mutex poisoning only happens when another thread panicked while holding the lock.
/// In spawn_blocking closures this is extremely unlikely, but we handle it properly.
fn lock_mutex<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!("Mutex was poisoned, recovering inner data");
            poisoned.into_inner()
        }
    }
}

use crate::BgLogsOutput;
use crate::ShellOutput;

pub struct ShellSession {
    cwd: String,
}

impl ShellSession {
    pub fn new(cwd: Option<&str>) -> Result<Self> {
        let cwd = cwd
            .map(|s| s.to_string())
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "/".to_string());

        Ok(Self { cwd })
    }

    pub async fn run(&self, command: &str, timeout_secs: u64) -> Result<ShellOutput> {
        debug!(
            "Running shell command: {} (cwd={}, timeout={}s)",
            command, self.cwd, timeout_secs
        );

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::task::spawn_blocking({
                let cwd = self.cwd.clone();
                let cmd = command.to_string();
                move || -> Result<ShellOutput> {
                    let child = if cfg!(windows) {
                        Command::new("powershell.exe")
                            .args(["-NoProfile", "-Command", &cmd])
                            .current_dir(&cwd)
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .spawn()
                    } else {
                        Command::new("sh")
                            .args(["-lc", &cmd])
                            .current_dir(&cwd)
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .spawn()
                    };

                    match child {
                        Ok(mut child) => {
                            let stdout = child
                                .stdout
                                .take()
                                .map(|stdout| {
                                    let reader = BufReader::new(stdout);
                                    reader
                                        .lines()
                                        .map_while(Result::ok)
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                })
                                .unwrap_or_default();

                            let stderr = child
                                .stderr
                                .take()
                                .map(|stderr| {
                                    let reader = BufReader::new(stderr);
                                    reader
                                        .lines()
                                        .map_while(Result::ok)
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                })
                                .unwrap_or_default();

                            let exit_code =
                                child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);

                            Ok(ShellOutput {
                                stdout,
                                stderr,
                                exit_code,
                                timed_out: false,
                            })
                        }
                        Err(e) => Ok(ShellOutput {
                            stdout: String::new(),
                            stderr: format!("Failed to spawn command: {}", e),
                            exit_code: -1,
                            timed_out: false,
                        }),
                    }
                }
            }),
        )
        .await;

        match output {
            Ok(Ok(result)) => Ok(result?),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Ok(ShellOutput {
                stdout: String::new(),
                stderr: format!("Command timed out after {} seconds", timeout_secs),
                exit_code: -1,
                timed_out: true,
            }),
        }
    }
}

pub struct ShellBgProcess {
    child: Arc<Mutex<Option<std::process::Child>>>,
    command: String,
    ring_buffer: Arc<Mutex<Vec<String>>>,
    next_offset: Arc<Mutex<usize>>,
}

impl ShellBgProcess {
    pub fn spawn(command: &str, cwd: Option<&str>) -> Result<Self> {
        info!("Spawning background process: {} (cwd={:?})", command, cwd);

        let mut child_cmd = if cfg!(windows) {
            let mut c = Command::new("powershell.exe");
            c.args(["-NoProfile", "-Command", command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-lc", command]);
            c
        };

        if let Some(dir) = cwd {
            child_cmd.current_dir(dir);
        }

        let mut child = child_cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn background process")?;

        let ring_buffer = Arc::new(Mutex::new(Vec::with_capacity(4096)));
        let next_offset = Arc::new(Mutex::new(0));

        if let Some(stdout) = child.stdout.take() {
            let buffer = ring_buffer.clone();
            let offset = next_offset.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    let mut buf = lock_mutex(&buffer);
                    buf.push(line);
                    let mut off = lock_mutex(&offset);
                    *off = buf.len();
                    if buf.len() > 4096 {
                        buf.drain(0..1024);
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let buffer = ring_buffer.clone();
            let offset = next_offset.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    let mut buf = lock_mutex(&buffer);
                    buf.push(format!("[stderr] {}", line));
                    let mut off = lock_mutex(&offset);
                    *off = buf.len();
                    if buf.len() > 4096 {
                        buf.drain(0..1024);
                    }
                }
            });
        }

        Ok(Self {
            child: Arc::new(Mutex::new(Some(child))),
            command: command.to_string(),
            ring_buffer,
            next_offset,
        })
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn is_running(&self) -> bool {
        if let Some(child) = lock_mutex(&self.child).as_mut() {
            child.try_wait().map(|s| s.is_none()).unwrap_or(false)
        } else {
            false
        }
    }

    pub fn logs(&self, since_offset: usize) -> Result<BgLogsOutput> {
        let buffer = lock_mutex(&self.ring_buffer);
        let offset = *lock_mutex(&self.next_offset);

        let lines: Vec<String> = buffer.iter().skip(since_offset).cloned().collect();
        let output = lines.join("\n");

        Ok(BgLogsOutput {
            stdout: output,
            next_offset: offset,
            dropped: if since_offset > 0 {
                since_offset.min(buffer.len())
            } else {
                0
            },
            finished: !self.is_running(),
        })
    }

    pub fn kill(&self) -> Result<()> {
        if let Some(child) = lock_mutex(&self.child).as_mut() {
            child.kill().context("Failed to kill background process")?;
        }
        Ok(())
    }
}
