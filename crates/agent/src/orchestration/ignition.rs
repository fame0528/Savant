use anyhow::{Context, Result};
use pqcrypto_dilithium::dilithium2;
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{error, info, warn};

use crate::manager::AgentManager;
use crate::swarm::{SwarmConfig, SwarmController};
use crate::watcher::SwarmWatcher;
use savant_core::bus::NexusBridge;
use savant_core::config::Config;
use savant_core::crypto::AgentKeyPair;
use savant_core::db::Storage;
use savant_gateway::server::start_gateway;
use savant_obsidian::{ColdStorageManager, OutboxWorker, VaultWatcher, VaultWriter};

/// Kills any process using the specified port.
/// On Windows, uses PowerShell. On Unix, uses lsof.
/// Best-effort: logs warnings on failure but doesn't block startup.
async fn kill_port_process(port: u16) {
    // Check if port is in use
    if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
        return; // Port is free, nothing to do
    }

    warn!("[ignition] Port {} is in use — attempting to free it", port);

    #[cfg(target_os = "windows")]
    {
        let _ = tokio::process::Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Get-NetTCPConnection -LocalPort {} -ErrorAction SilentlyContinue | ForEach-Object {{ Stop-Process -Id $_.OwningProcess -Force }}",
                    port
                ),
            ])
            .output()
            .await;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = tokio::process::Command::new("sh")
            .args([
                "-c",
                &format!("lsof -ti :{} | xargs kill -9 2>/dev/null", port),
            ])
            .output()
            .await;
    }

    // Wait a moment for the process to release the port
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
        info!("[ignition] Port {} freed successfully", port);
    } else {
        warn!(
            "[ignition] Port {} still in use after cleanup attempt",
            port
        );
    }
}

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
            config_file: Some(std::path::PathBuf::from(config_path.unwrap_or("config/savant.toml"))),
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

        // 7. Obsidian Vault Projection Worker (always active by default)
        if config.obsidian.enabled {
            let obsidian_config = config.obsidian.clone();
            let obsidian_enclave = swarm.engine().enclave();
            let obsidian_nexus = nexus.clone();
            let agents_path = config.resolve_path(&config.system.agents_path);

            let vault_path = obsidian_config
                .resolved_vault_path(&agents_path);

            // Outbox worker owns VaultWriter + ColdStorageManager
            let writer = VaultWriter::new(
                vault_path.clone(),
                Some(Arc::clone(&obsidian_enclave)),
                obsidian_config.clone(),
                "savant".to_string(),
            );
            let cold_storage = ColdStorageManager::new(vault_path.clone(), obsidian_config.clone());
            let (outbox_shutdown_tx, outbox_shutdown_rx) = watch::channel(false);
            let outbox = OutboxWorker::new(
                vault_path.clone(),
                writer,
                cold_storage,
                obsidian_config.clone(),
                Some(Arc::clone(&obsidian_enclave)),
                agents_path.clone(),
                outbox_shutdown_rx,
            );
            let outbox_handle = tokio::spawn({
                let vault_path = vault_path.clone();
                async move {
                    info!(
                        "[obsidian] Vault projection worker starting at {}",
                        vault_path.display()
                    );
                    outbox.run().await;
                }
            });

            // Vault watcher owns its own reference to enclave + nexus
            let (watcher_shutdown_tx, watcher_shutdown_rx) = watch::channel(false);
            let watcher = VaultWatcher::new(
                vault_path.clone(),
                obsidian_config,
                Some(Arc::clone(&obsidian_nexus)),
                Some(Arc::clone(&obsidian_enclave)),
                watcher_shutdown_rx,
            );
            let watcher_handle = tokio::spawn({
                let vault_path = vault_path.clone();
                async move {
                    info!("[obsidian] Vault watcher starting at {}", vault_path.display());
                    if let Err(e) = watcher.run().await {
                        warn!("[obsidian] Vault watcher error: {e}");
                    }
                }
            });

            // Store shutdown senders so they can be used for graceful shutdown
            let _ = (outbox_shutdown_tx, watcher_shutdown_tx, outbox_handle, watcher_handle);

            info!("[obsidian] Vault projection workers enabled at {}", vault_path.display());
        }

        // 8. Gateway (Async Background)
        // Kill any stale process on the gateway port before starting
        kill_port_process(config.server.port).await;
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
