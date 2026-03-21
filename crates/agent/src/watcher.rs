use crate::manager::AgentManager;
use crate::swarm::SwarmController;
use notify_debouncer_mini::{new_debouncer, notify::*, DebounceEventResult};
use savant_core::bus::NexusBridge;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;

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

    pub async fn start(self: Arc<Self>) -> anyhow::Result<()> {
        let (tx, mut rx) = mpsc::channel(32);

        let mut debouncer = new_debouncer(
            std::time::Duration::from_millis(1000),
            move |res: DebounceEventResult| {
                if let Ok(events) = res {
                    for event in events {
                        let _ = tx.blocking_send(event.path);
                    }
                }
            },
        )?;

        let workspaces = self.manager._config.project_root.join("workspaces");
        if !workspaces.exists() {
            let _ = std::fs::create_dir_all(&workspaces);
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

        while let Some(path) = rx.recv().await {
            // Identify which agent was touched
            if let Some(agent_workspace) = self.find_workspace_root(&path) {
                // Ignore changes to agent.json itself to prevent loops
                if path.ends_with("agent.json") {
                    continue;
                }

                tracing::info!("🔄 Hot-reload detected: {}", agent_workspace.display());

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
                                let _ = self
                                    .nexus
                                    .publish("agents.discovered", &discovery_event.to_string())
                                    .await;
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
                    // We need a way to find the agent_id from the path if agent.json is gone
                    // But our ensure_stable_id created it, so if the whole folder is gone,
                    // we might need to evacuate all agents that were in that folder.
                    // For now, reload all discovery is safer but heavier.
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
