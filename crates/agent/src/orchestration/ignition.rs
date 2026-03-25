use anyhow::{Context, Result};
use pqcrypto_dilithium::dilithium2;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::manager::AgentManager;
use crate::swarm::{SwarmConfig, SwarmController};
use crate::watcher::SwarmWatcher;
use savant_core::bus::NexusBridge;
use savant_core::config::Config;
use savant_core::crypto::AgentKeyPair;
use savant_core::db::Storage;
use savant_gateway::server::start_gateway;

/// 🧬 Savant Ignition Outcome
/// Holds the live handlers for the entire swarm ecosystem.
pub struct SwarmIgnition {
    pub config: Config,
    pub nexus: Arc<NexusBridge>,
    pub storage: Arc<Storage>,
    pub swarm: Arc<SwarmController>,
    pub manager: Arc<AgentManager>,
}

/// 🚀 Swarm Ignition Service
/// Orchestrates the complex startup sequence for the Savant environment.
pub struct IgnitionService;

impl IgnitionService {
    /// Ignites the swarm using the provided configuration.
    pub async fn ignite(config_path: Option<&str>) -> Result<SwarmIgnition> {
        info!("🧬 Initializing Savant Swarm Substrate...");

        // 1. Configuration
        let config = Config::load_from(config_path).context("Failed to load configuration")?;
        info!(
            "✅ Configuration substrate successfully initialized at {}",
            config.project_root.display()
        );

        // 2. Crypto
        let master_key = AgentKeyPair::ensure_master_key().context("Master key failure")?;
        let root_authority = master_key
            .get_verifying_key()
            .context("Failed to derive root authority")?;
        let signing_key = master_key
            .get_signing_key()
            .context("Failed to derive signing key")?;
        let (pqc_authority, pqc_signing_key) = dilithium2::keypair();
        info!("🔐 Cryptographic identity established");

        // 3. Event Bus
        let nexus = Arc::new(NexusBridge::new());
        info!("🌐 Nexus event bus operational");

        // 4. Storage
        let db_path = config.resolve_path(&config.system.db_path);
        info!("💾 Synchronizing storage at: {}", db_path.display());
        let storage =
            Arc::new(Storage::with_defaults(db_path).context("Storage initialization failed")?);

        // 5. Agent Discovery
        let manager = Arc::new(AgentManager::new(config.clone()));
        info!("🔍 Starting agent discovery sequence...");
        let discovered_agents = manager
            .discover_agents()
            .await
            .context("Agent discovery failed")?;

        let mut agent_metadata = Vec::new();
        if discovered_agents.is_empty() {
            warn!("🔍 No agents found in workspace clusters. Check your workspaces directory.");
        } else {
            info!(
                "✅ Discovered {} agents for deployment",
                discovered_agents.len()
            );
            for a in &discovered_agents {
                info!("   - Agent: {} ({})", a.agent_name, a.agent_id);
                agent_metadata.push(serde_json::json!({
                    "id": a.agent_id,
                    "name": a.agent_name,
                    "status": "Active",
                    "role": "Agent",
                    "image": a.identity.as_ref().and_then(|i| i.image.clone())
                }));
            }
        }

        // Sync initial discovery state to bus
        let discovery_event = serde_json::json!({
            "status": "SWARM_IGNITED",
            "agents": agent_metadata
        });

        // Populate system.agents in shared memory and publish event
        nexus
            .update_state("system.agents".to_string(), discovery_event.to_string())
            .await;
        if let Err(e) = nexus
            .publish("agents.discovered", &discovery_event.to_string())
            .await
        {
            tracing::warn!(
                "[agent::ignition] Failed to publish agents.discovered event: {}",
                e
            );
        }

        // 6. Swarm Controller
        let swarm_config = SwarmConfig {
            workspace_root: config.resolve_path(&config.system.agents_path),
            memory_db_path: config.resolve_path("./data/memory"),
            skills_path: config.resolve_path("./skills"),
            blackboard_name: "savant_swarm".into(),
            collective_name: "savant_collective".into(),
        };

        let swarm = SwarmController::new(
            swarm_config,
            discovered_agents,
            storage.clone(),
            manager.clone(),
            nexus.clone(),
            root_authority,
            signing_key,
            pqc_authority,
            pqc_signing_key,
            config.mcp.servers.clone(),
        )
        .await
        .context("Swarm Controller ignition failed")?;

        let swarm = Arc::new(swarm);
        info!("🚀 Swarm Controller online and synchronized");

        // 7. Gateway (Async Background)
        let g_config = config.clone();
        let g_nexus = nexus.clone();
        let g_storage = storage.clone();
        tokio::spawn(async move {
            if let Err(e) = start_gateway(g_config, g_nexus, g_storage).await {
                error!("❌ Gateway crash: {}", e);
            }
        });

        // 8. Swarm Ignition & Watcher
        let s_swarm = swarm.clone();
        let s_manager = manager.clone();
        let s_nexus = nexus.clone();
        tokio::spawn(async move {
            s_swarm.ignite().await;
            let watcher = Arc::new(SwarmWatcher::new(s_swarm, s_manager, s_nexus));
            if let Err(e) = watcher.start().await {
                error!("🔭 SwarmWatcher error: {}", e);
            }
        });

        Ok(SwarmIgnition {
            config,
            nexus,
            storage,
            swarm,
            manager,
        })
    }
}
