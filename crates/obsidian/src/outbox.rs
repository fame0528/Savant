use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::watch;
use tracing::{debug, info, warn};

use savant_memory::engine::MemoryEnclave;

use crate::cold_storage::ColdStorageManager;
use crate::config::ObsidianConfig;
use crate::writer::VaultWriter;

/// Drives periodic vault projection by polling the outbox cursor.
///
/// Uses a cursor-based approach: on each tick, the worker queries the LSM for current
/// state statistics and compares against the last-projected state stored in a cursor file
/// at `{vault_path}/.cursor.json`. If the state has changed, a full projection is triggered.
///
/// This avoids the dual-write problem entirely: the vault writer always reads from the
/// canonically consistent LSM, and all file writes are atomic via tempfile+rename.
pub struct OutboxWorker {
    vault_path: PathBuf,
    writer: VaultWriter,
    cold_storage: ColdStorageManager,
    config: ObsidianConfig,
    enclave: Option<Arc<MemoryEnclave>>,
    workspace_root: PathBuf,
    shutdown: watch::Receiver<bool>,
}

impl OutboxWorker {
    pub fn new(
        vault_path: PathBuf,
        writer: VaultWriter,
        cold_storage: ColdStorageManager,
        config: ObsidianConfig,
        enclave: Option<Arc<MemoryEnclave>>,
        workspace_root: PathBuf,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            vault_path,
            writer,
            cold_storage,
            config,
            enclave,
            workspace_root,
            shutdown,
        }
    }

    /// Runs the outbox drain loop. Spawn this as a tokio task.
    /// Polls on the configured interval and triggers projection when state changes.
    pub async fn run(&self) {
        let interval_secs = self.config.sync_interval_secs.max(10);
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        interval.tick().await; // skip first immediate tick

        // Load the last-known cursor on startup
        let cursor = CursorState::load(&self.vault_path);

        info!(
            "[obsidian] Outbox worker started (interval={interval_secs}s, \
             vault={vault})",
            vault = self.vault_path.display(),
        );

        loop {
            let mut shutdown = self.shutdown.clone();
            tokio::select! {
                _ = interval.tick() => {}
                _ = shutdown.changed() => {
                    info!("[obsidian] Outbox worker shutting down");
                    return;
                }
            }

            // Compare current state against cursor
            let current = self.snapshot_state();
            if !cursor.has_changed(&current) {
                debug!("[obsidian] No state change since last sync; skipping");
                continue;
            }

            debug!("[obsidian] State change detected; running projection");
            match self.writer.run_full_sync(&self.workspace_root) {
                Ok(stats) => {
                    CursorState {
                        session_count: stats.session_count,
                        memory_count: stats.memory_count,
                        vector_count: stats.vector_count,
                        mutation_count: stats.mutation_count,
                        vault_file_count: stats.vault_file_count as u64,
                        timestamp: Utc::now().timestamp(),
                    }
                    .save(&self.vault_path);

                    // Run cold storage check after successful sync
                    if let Err(e) = self.cold_storage.run(&self.writer) {
                        warn!("[obsidian] Cold storage check failed: {e}");
                    }

                    info!(
                        "[obsidian] Sync complete: {files} files, \
                         {sessions} sessions, {memories} memories, {vectors} vectors",
                        files = stats.vault_file_count,
                        sessions = stats.session_count,
                        memories = stats.memory_count,
                        vectors = stats.vector_count,
                    );
                }
                Err(e) => {
                    warn!("[obsidian] Vault sync failed: {e}");
                }
            }
        }
    }

    fn snapshot_state(&self) -> StateSnapshot {
        let mut snapshot = StateSnapshot::default();
        if let Some(enclave) = &self.enclave {
            let lsm = enclave.lsm();
            if let Ok(s) = lsm.stats() {
                snapshot.session_count = s.total_sessions;
                snapshot.memory_count = s.total_messages;
            }
            snapshot.vector_count = enclave.vector_count() as u64;
        }
        snapshot
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
struct StateSnapshot {
    session_count: u64,
    memory_count: u64,
    vector_count: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[derive(Default)]
struct CursorState {
    session_count: u64,
    memory_count: u64,
    vector_count: u64,
    mutation_count: u64,
    vault_file_count: u64,
    timestamp: i64,
}

impl CursorState {
    fn load(vault_path: &Path) -> Self {
        let path = vault_path.join(".cursor.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cursor) = serde_json::from_str::<CursorState>(&content) {
                    return cursor;
                }
            }
        }
        CursorState::default()
    }

    fn save(&self, vault_path: &Path) {
        let path = vault_path.join(".cursor.json");
        if let Ok(content) = serde_json::to_string(self) {
            let tmp = path.with_extension("tmp");
            if let Ok(mut f) = std::fs::File::create(&tmp) {
                use std::io::Write;
                let _ = f.write_all(content.as_bytes());
                let _ = f.sync_all();
                let _ = std::fs::rename(&tmp, &path);
            }
        }
    }

    fn has_changed(&self, state: &StateSnapshot) -> bool {
        self.session_count != state.session_count
            || self.memory_count != state.memory_count
            || self.vector_count != state.vector_count
    }
}

