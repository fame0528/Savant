use crate::budget::TokenBudget;
use crate::context::ContextAssembler;
use savant_core::traits::{LlmProvider, MemoryBackend, Tool};
use savant_core::types::AgentIdentity;
use std::sync::Arc;
use savant_cognitive::DspPredictor;
use savant_echo::{HotSwappableRegistry, ComponentMetrics};
use savant_ipc::CollectiveBlackboard;
use crate::plugins::WasmToolHost;

pub mod events;
pub mod reactor;
pub mod stream;

pub use events::AgentEvent;
use savant_core::types::ChatMessage;

#[derive(Debug, Clone, Default)]
pub struct HeuristicState {
    pub failures: usize,
    pub retries: usize,
    pub depth: u32,
    pub last_stable_checkpoint: Option<Vec<ChatMessage>>,
}

pub struct AgentLoop<M: MemoryBackend> {
    pub(crate) agent_id: String,
    pub(crate) agent_id_hash: u64,
    pub(crate) provider: Box<dyn LlmProvider>,
    pub(crate) fallback_provider: Option<Box<dyn LlmProvider>>,
    pub(crate) memory: M,
    pub(crate) tools: Vec<Arc<dyn Tool>>,
    pub(crate) context: ContextAssembler,
    pub(crate) plugin_host: Option<Arc<crate::plugins::WasmPluginHost>>,
    pub(crate) plugins: Vec<wasmtime::component::Component>,
    pub(crate) security_token: Option<savant_security::AgentToken>,
    pub(crate) security_authority: Option<Arc<savant_security::SecurityAuthority>>,
    pub(crate) predictor: DspPredictor,
    pub(crate) echo_registry: Option<Arc<HotSwappableRegistry>>,
    pub(crate) echo_metrics: Option<Arc<ComponentMetrics>>,
    pub(crate) echo_host: Option<Arc<WasmToolHost>>,
    pub(crate) collective_blackboard: Option<Arc<CollectiveBlackboard>>,
    pub(crate) hyper_causal: Arc<crate::orchestration::branching::HyperCausalEngine>,
    pub(crate) agent_index: u8,
    pub(crate) max_parallel_tools: usize,
    pub(crate) heuristic: HeuristicState,
}

impl<M: MemoryBackend> AgentLoop<M> {
    pub fn new(
        agent_id: String,
        provider: Box<dyn LlmProvider>,
        memory: M,
        tools: Vec<Arc<dyn Tool>>,
        identity: AgentIdentity,
    ) -> Self {
        let mut skills_summary = String::from("Available Tools:\n");
        for tool in &tools {
            skills_summary.push_str(&format!("- {}: {}\n", tool.name(), tool.description()));
        }
        let skills_list = if tools.is_empty() {
            None
        } else {
            Some(skills_summary)
        };

        let agent_id_hash = xxhash_rust::xxh3::xxh3_64(agent_id.as_bytes());

        Self {
            agent_id,
            agent_id_hash,
            provider,
            fallback_provider: None,
            memory,
            tools,
            context: ContextAssembler::new(identity, TokenBudget::new(256000), skills_list),
            plugin_host: None,
            plugins: Vec::new(),
            security_token: None,
            security_authority: None,
            predictor: DspPredictor::default(),
            echo_registry: None,
            echo_metrics: None,
            echo_host: None,
            collective_blackboard: None,
            hyper_causal: Arc::new(crate::orchestration::branching::HyperCausalEngine::default()),
            agent_index: 0,
            max_parallel_tools: 5,
            heuristic: HeuristicState::default(),
        }
    }

    pub fn with_plugins(
        mut self, 
        host: Arc<crate::plugins::WasmPluginHost>, 
        plugins: Vec<wasmtime::component::Component>,
        token: Option<savant_security::AgentToken>,
    ) -> Self {
        self.plugin_host = Some(host);
        self.plugins = plugins;
        self.security_token = token;
        self
    }

    pub fn with_echo(
        mut self,
        registry: Arc<HotSwappableRegistry>,
        metrics: Arc<ComponentMetrics>,
        host: Arc<WasmToolHost>,
    ) -> Self {
        self.echo_registry = Some(registry);
        self.echo_metrics = Some(metrics);
        self.echo_host = Some(host);
        self
    }

    pub fn with_fallback(mut self, provider: Box<dyn LlmProvider>) -> Self {
        self.fallback_provider = Some(provider);
        self
    }

    pub fn with_collective(
        mut self,
        blackboard: Arc<CollectiveBlackboard>,
        index: u8,
    ) -> Self {
        self.collective_blackboard = Some(blackboard);
        self.agent_index = index;
        self
    }

    pub fn with_security_authority(
        mut self,
        authority: Arc<savant_security::SecurityAuthority>,
    ) -> Self {
        self.security_authority = Some(authority);
        self
    }

    pub fn with_hyper_causal(
        mut self,
        engine: crate::orchestration::branching::HyperCausalEngine,
    ) -> Self {
        self.hyper_causal = Arc::new(engine);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use savant_core::types::{ChatMessage, AgentIdentity};
    use savant_core::traits::{LlmProvider, MemoryBackend, Tool};
    use savant_core::error::SavantError;
    use async_trait::async_trait;
    use serde_json::Value;
    use tokio::sync::Mutex;
    use std::pin::Pin;
    use futures::Stream;
    use tokio_util::sync::CancellationToken;
    use futures::stream::StreamExt;

    struct MockTool {
        name: String,
        executed: Arc<Mutex<u32>>,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str { &self.name }
        fn description(&self) -> &str { "Mock" }
        async fn execute(&self, _args: Value) -> Result<String, SavantError> {
            let mut count = self.executed.lock().await;
            *count += 1;
            Ok(format!("{} executed", self.name))
        }
    }

    struct MockLlm {
        responses: Vec<String>,
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn stream_completion(&self, _messages: Vec<ChatMessage>) -> Result<Pin<Box<dyn Stream<Item = Result<savant_core::types::ChatChunk, SavantError>> + Send>>, SavantError> {
            let mut count = self.call_count.lock().await;
            let response = if *count < self.responses.len() {
                self.responses[*count].clone()
            } else {
                "Final Answer: Done".to_string()
            };
            *count += 1;
            let chunk = savant_core::types::ChatChunk {
                agent_name: "test".to_string(),
                agent_id: "test".to_string(),
                content: response,
                is_final: true,
                session_id: None,
                channel: savant_core::types::AgentOutputChannel::Chat,
            };
            Ok(Box::pin(futures::stream::iter(vec![Ok(chunk)])))
        }
    }

    struct MockMemory;
    #[async_trait]
    impl MemoryBackend for MockMemory {
        async fn store(&self, _agent_id: &str, _msg: &ChatMessage) -> Result<(), SavantError> { Ok(()) }
        async fn retrieve(&self, _agent_id: &str, _query: &str, _limit: usize) -> Result<Vec<ChatMessage>, SavantError> { Ok(vec![]) }
        async fn consolidate(&self, _agent_id: &str) -> Result<(), SavantError> { Ok(()) }
    }

    #[tokio::test]
    async fn test_speculative_chaining() {
        let executed_count = Arc::new(Mutex::new(0));
        let tool1 = Arc::new(MockTool { name: "Tool1".into(), executed: executed_count.clone() });
        let tool2 = Arc::new(MockTool { name: "Tool2".into(), executed: executed_count.clone() });
        
        let provider = Box::new(MockLlm {
            responses: vec!["Thought: Doing two things.\nAction: Tool1[arg1]\nAction: Tool2[arg2]".to_string()],
            call_count: Arc::new(Mutex::new(0)),
        });

        let mut agent = AgentLoop::new(
            "test_agent".into(),
            provider,
            MockMemory,
            vec![tool1, tool2],
            AgentIdentity::default(),
        ).with_hyper_causal(crate::orchestration::branching::HyperCausalEngine::new(1));

        let mut stream = agent.run("start".into(), None, CancellationToken::new());
        while let Some(res) = stream.next().await {
            if let Err(e) = res {
                panic!("Stream error: {:?}", e);
            }
        }

        let count = executed_count.lock().await;
        assert_eq!(*count, 2, "Both tools in the speculative chain should have executed");
    }
}

#[cfg(test)]
mod heuristic_tests;
