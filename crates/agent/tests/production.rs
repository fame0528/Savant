#![allow(clippy::disallowed_methods)]
use std::sync::Arc;
use savant_core::types::{AgentConfig, ChatMessage, ChatChunk, ModelProvider, AgentIdentity};
use savant_core::traits::LlmProvider;
use savant_core::error::SavantError;
use savant_agent::swarm::SwarmController;
use savant_core::bus::NexusBridge;
use savant_core::db::Storage;
use savant_agent::manager::AgentManager;
use futures::stream::{self, Stream};
use std::pin::Pin;
use async_trait::async_trait;
use std::collections::HashMap;

#[allow(dead_code)]
struct MockLlmProvider;

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn stream_completion(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError> {
        let chunk = ChatChunk {
            agent_name: "Mock".to_string(),
            agent_id: "mock-id".to_string(),
            content: "Mock response".to_string(),
            is_final: true,
        };
        Ok(Box::pin(stream::iter(vec![Ok(chunk)])))
    }
}

#[tokio::test]
async fn test_production_swarm_initialization_50_agents() {
    // 1. Setup temp environment
    let base_temp = std::env::temp_dir().join(format!("savant_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&base_temp).expect("Failed to create base temp dir");
    
    let storage_path = base_temp.join("test.db");
    let skills_path = base_temp.join("skills");
    let workspace_path = base_temp.join("workspace");
    let memory_path = base_temp.join("data/memory");
    
    std::fs::create_dir_all(&skills_path).unwrap();
    std::fs::create_dir_all(&workspace_path).unwrap();
    std::fs::create_dir_all(&memory_path).unwrap();

    // 2. Mock keys
    let mut rng = rand::thread_rng();
    let signing_key = ed25519_dalek::SigningKey::generate(&mut rng);
    let root_authority = signing_key.verifying_key();

    // 3. Create dependencies
    let nexus = Arc::new(NexusBridge::new());
    let storage = Arc::new(Storage::new(storage_path).expect("Failed to open test storage"));
    storage.init_schema().expect("Failed to init schema");
    
    let config = Config::default();
    let manager = Arc::new(AgentManager::new(config));

    // 4. Create 50 agents
    let mut agents = Vec::new();
    for i in 0..50 {
        let agent_id = format!("agent_{}", i);
        agents.push(AgentConfig {
            agent_id: agent_id.clone(),
            agent_name: agent_id,
            model_provider: ModelProvider::OpenRouter,
            api_key: Some("mock_key".to_string()),
            env_vars: HashMap::new(),
            system_prompt: "You are a test agent.".to_string(),
            model: Some("anthropic/claude-3-sonnet".to_string()),
            heartbeat_interval: 10,
            allowed_skills: Vec::new(),
            workspace_path: workspace_path.join(format!("agent_{}", i)),
            identity: Some(AgentIdentity::default()),
            parent_id: None,
            session_id: Some("test-session".to_string()),
        });
    }

    // 5. Initialize Controller
    let controller = SwarmController::new(
        agents,
        storage,
        manager,
        nexus,
        root_authority,
        signing_key,
    ).expect("Failed to create SwarmController");

    // 6. Ignite
    controller.ignite().await;

    // 7. Verify health (Wait for agents to boot)
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let dead_agents = controller.check_swarm_health().await;
    assert!(dead_agents.is_empty(), "Dead agents detected: {:?}", dead_agents);
    
    // 8. Verify IPC (Blackboard existence)
    // In a real scenario, we'd check if the agents are writing to the blackboard
}

#[tokio::test]
async fn test_agent_panic_recovery_logic() {
    // This test would verify that the SwarmController handles agent task completion/failure
    // Since SwarmController current doesn't auto-restart, we verify evacuation works.
    
    let base_temp = std::env::temp_dir().join(format!("savant_panic_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&base_temp).unwrap();
    
    let signing_key = ed25519_dalek::SigningKey::generate(&mut rand::thread_rng());
    let root_authority = signing_key.verifying_key();
    
    let controller = SwarmController::new(
        vec![AgentConfig {
            agent_id: "unstable_agent".to_string(),
            agent_name: "Unstable".to_string(),
            model_provider: ModelProvider::OpenRouter,
            api_key: Some("mock".to_string()),
            env_vars: HashMap::new(),
            system_prompt: "test".to_string(),
            model: None,
            heartbeat_interval: 5,
            allowed_skills: Vec::new(),
            workspace_path: base_temp.join("unstable"),
            identity: None,
            parent_id: None,
            session_id: None,
        }],
        Arc::new(Storage::new(base_temp.join("panic.db")).expect("Failed to open panic storage")),
        Arc::new(AgentManager::new(Config::default())),
        Arc::new(NexusBridge::new()),
        root_authority,
        signing_key,
    ).unwrap();

    controller.ignite().await;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    controller.evacuate_agent("unstable_agent").await;
    
    let dead = controller.check_swarm_health().await;
    assert!(dead.contains(&"unstable_agent".to_string()));
}

#[tokio::test]
async fn test_500_agent_initialization_scaling() {
    // Audit-grade scaling verification
    let base_temp = std::env::temp_dir().join(format!("savant_scale_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&base_temp).unwrap();
    
    let storage_path = base_temp.join("scale.db");
    
    let mut rng = rand::thread_rng();
    let signing_key = ed25519_dalek::SigningKey::generate(&mut rng);
    let root_authority = signing_key.verifying_key();

    let storage = Arc::new(Storage::new(storage_path).expect("Failed to open scale storage"));
    storage.init_schema().expect("Failed to init schema");
    
    let nexus = Arc::new(NexusBridge::new());
    let manager = Arc::new(AgentManager::new(Config::default()));

    let mut agents = Vec::new();
    for i in 0..500 {
        let agent_id = format!("scale_agent_{}", i);
        agents.push(AgentConfig {
            agent_id: agent_id.clone(),
            agent_name: agent_id,
            model_provider: ModelProvider::OpenRouter,
            api_key: Some("mock".to_string()),
            env_vars: HashMap::new(),
            system_prompt: "Scale Test".to_string(),
            model: None,
            heartbeat_interval: 10,
            allowed_skills: Vec::new(),
            workspace_path: base_temp.join(format!("agent_{}", i)),
            identity: None,
            parent_id: None,
            session_id: Some("scale-session".to_string()),
        });
    }

    let controller = SwarmController::new(
        agents,
        storage,
        manager,
        nexus,
        root_authority,
        signing_key,
    ).expect("Failed to create Scale Controller");

    controller.ignite().await;
    
    // Scaling target: <5s for 500 agents on standard SSD
    tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
    
    let dead = controller.check_swarm_health().await;
    assert!(dead.is_empty(), "Scaling failure: agents failed to ignite at 500 count: {:?}", dead);
}
