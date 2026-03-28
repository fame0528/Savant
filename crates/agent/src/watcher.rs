use crate::manager::AgentManager;
use crate::swarm::SwarmController;
use notify_debouncer_mini::{new_debouncer, notify::*, DebounceEventResult};
use savant_core::bus::NexusBridge;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

/// Files that the agent writes to during normal operation.
/// Changes to these files should NOT trigger hot-reload.
const AGENT_INTERNAL_FILES: &[&str] = &[
    "SOUL.md",
    "AGENTS.md",
    "LEARNINGS.md",
    "DEV-SESSION-STATE.md",
    "CONTEXT.md",
];

/// Minimum cooldown between hot-reload triggers (seconds).
/// Prevents boot-reboot loops when the agent writes files during startup.
const HOT_RELOAD_COOLDOWN_SECS: u64 = 15;

pub struct SwarmWatcher {
    swarm: Arc<SwarmController>,
    manager: Arc<AgentManager>,
    nexus: Arc<NexusBridge>,
}

impl SwarmWatcher {
    pub fn new(
        swarm: Arc<SwarmController>,
        manager: Arc<AgentManager>,
        nexus: Arc<NexusBridge>,
    ) -> Self {
        Self {
            swarm,
            manager,
            nexus,
        }
    }

    /// Returns true if the file change should be ignored (agent-internal file).
    fn should_ignore(path: &Path) -> bool {
        // Ignore WAL directory (hidden directories starting with '.')
        if let Some(parent) = path.parent() {
            let parent_name = parent.file_name().map(|n| n.to_string_lossy());
            if parent_name
                .as_ref()
                .map(|n| n.starts_with('.'))
                .unwrap_or(false)
            {
                return true;
            }
        }

        // Ignore agent-internal files (SOUL.md, AGENTS.md, etc.)
        if let Some(file_name) = path.file_name() {
            let name = file_name.to_string_lossy();
            if AGENT_INTERNAL_FILES.iter().any(|f| name == *f) {
                return true;
            }
        }

        // Ignore agent.json (already handled, but explicit for clarity)
        if path.ends_with("agent.json") {
            return true;
        }

        false
    }

    pub async fn start(self: Arc<Self>) -> anyhow::Result<()> {
        let (tx, mut rx) = mpsc::channel(32);

        let mut debouncer = new_debouncer(
            std::time::Duration::from_millis(2000),
            move |res: DebounceEventResult| {
                if let Ok(events) = res {
                    for event in events {
                        if let Err(e) = tx.blocking_send(event.path) {
                            tracing::warn!("[agent::watcher] Failed to send file event: {}", e);
                        }
                    }
                }
            },
        )?;

        let workspaces = self.manager._config.project_root.join("workspaces");
        if !workspaces.exists() {
            if let Err(e) = std::fs::create_dir_all(&workspaces) {
                tracing::warn!(
                    "[agent::watcher] Failed to create workspaces directory: {}",
                    e
                );
            }
        }
        let workspaces = workspaces.canonicalize().unwrap_or(workspaces);

        debouncer
            .watcher()
            .watch(&workspaces, RecursiveMode::Recursive)?;

        tracing::info!(
            "🔭 SwarmWatcher activated: Monitoring {}...",
            workspaces.display()
        );

        // Keep debouncer alive
        let _debouncer = debouncer;

        // Cooldown timer: prevents rapid hot-reload cycles
        let mut last_reload: Option<Instant> = None;

        while let Some(path) = rx.recv().await {
            // Identify which agent was touched
            if let Some(agent_workspace) = self.find_workspace_root(&path) {
                // Filter: ignore agent-internal files (SOUL.md, AGENTS.md, WAL, etc.)
                if Self::should_ignore(&path) {
                    continue;
                }

                // Cooldown: prevent hot-reload if one happened recently
                if let Some(last) = last_reload {
                    let elapsed = last.elapsed().as_secs();
                    if elapsed < HOT_RELOAD_COOLDOWN_SECS {
                        tracing::debug!(
                            "🔭 SwarmWatcher: Hot-reload suppressed (cooldown: {}s remaining, triggered by: {})",
                            HOT_RELOAD_COOLDOWN_SECS - elapsed,
                            path.display()
                        );
                        continue;
                    }
                }

                tracing::info!(
                    "🔄 Hot-reload detected: {} (triggered by: {})",
                    agent_workspace.display(),
                    path.display()
                );
                last_reload = Some(Instant::now());

                // Re-discover and re-spawn
                if agent_workspace.exists() {
                    match self.manager.registry.load_agent(&agent_workspace) {
                        Ok(agent_cfg) => {
                            self.swarm.spawn_agent(agent_cfg).await;

                            // Re-broadcast discovery
                            if let Ok(agents) = self.manager.discover_agents().await {
                                let discovery_event = serde_json::json!({
                                    "status": "SWARM_HOT_RELOAD",
                                    "agents": agents.iter().map(|a| serde_json::json!({
                                        "id": a.agent_id,
                                        "name": a.agent_name,
                                        "status": "Active",
                                        "role": "Agent",
                                        "image": a.identity.as_ref().and_then(|i| i.image.clone())
                                    })).collect::<Vec<_>>()
                                });
                                if let Err(e) = self
                                    .nexus
                                    .publish("agents.discovered", &discovery_event.to_string())
                                    .await
                                {
                                    tracing::warn!("[agent::watcher] Failed to publish agent discovery event: {}", e);
                                }
                                self.nexus
                                    .update_state(
                                        "system.agents".to_string(),
                                        discovery_event.to_string(),
                                    )
                                    .await;
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "⚠️ Failed to reload agent from {}: {}",
                                agent_workspace.display(),
                                e
                            );
                        }
                    }
                } else {
                    // Evacuate if the directory was deleted
                }
            }
        }

        Ok(())
    }

    fn find_workspace_root(&self, path: &Path) -> Option<PathBuf> {
        let mut current = path;
        // Search upwards until we find a directory inside 'workspaces'
        while let Some(parent) = current.parent() {
            if parent
                .file_name()
                .map(|n| n.to_string_lossy() == "workspaces")
                .unwrap_or(false)
            {
                return Some(current.to_path_buf());
            }
            current = parent;
        }
        None
    }
}
