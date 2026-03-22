use crate::budget::TokenBudget;
use crate::context::ContextAssembler;
use crate::plugins::WasmToolHost;
use savant_cognitive::DspPredictor;
use savant_core::traits::{LlmProvider, MemoryBackend, Tool, VisionProvider};
use savant_core::types::AgentIdentity;
use savant_echo::{ComponentMetrics, HotSwappableRegistry};
use savant_ipc::CollectiveBlackboard;
use std::sync::Arc;

pub mod compaction;
pub mod events;
pub mod reactor;
pub mod self_repair;
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

pub enum LoopSignal {
    Continue,
    Terminate,
}

pub enum LoopOutcome {
    Success(String),
    Failure(String),
}

pub struct ChatResponse {
    pub content: String,
    pub tool_calls: Vec<savant_core::types::ProviderToolCall>,
}

pub enum TextAction {
    ParseActions,
    Ignore,
}

pub struct LoopContext<'a, M: MemoryBackend> {
    pub loop_state: &'a mut AgentLoop<M>,
    pub trace: &'a mut String,
}

#[async_trait::async_trait]
pub trait LoopDelegate<M: MemoryBackend>: Send + Sync {
    async fn check_signals(&self) -> LoopSignal;
    async fn before_llm_call(&self, ctx: &mut LoopContext<'_, M>) -> Option<LoopOutcome>;
    async fn call_llm(
        &self,
        ctx: &mut LoopContext<'_, M>,
    ) -> Result<ChatResponse, savant_core::error::SavantError>;
    async fn handle_text_response(&self, text: &str, ctx: &mut LoopContext<'_, M>) -> TextAction;
    async fn execute_tool_calls(
        &self,
        calls: Vec<savant_core::types::ProviderToolCall>,
        ctx: &mut LoopContext<'_, M>,
    ) -> Result<Option<LoopOutcome>, savant_core::error::SavantError>;
}

pub struct ChatDelegate;
#[async_trait::async_trait]
impl<M: MemoryBackend> LoopDelegate<M> for ChatDelegate {
    async fn check_signals(&self) -> LoopSignal {
        LoopSignal::Continue
    }
    async fn before_llm_call(&self, _ctx: &mut LoopContext<'_, M>) -> Option<LoopOutcome> {
        None
    }
    async fn call_llm(
        &self,
        _ctx: &mut LoopContext<'_, M>,
    ) -> Result<ChatResponse, savant_core::error::SavantError> {
        Ok(ChatResponse {
            content: String::new(),
            tool_calls: vec![],
        })
    }
    async fn handle_text_response(&self, _text: &str, _ctx: &mut LoopContext<'_, M>) -> TextAction {
        TextAction::ParseActions
    }
    async fn execute_tool_calls(
        &self,
        _calls: Vec<savant_core::types::ProviderToolCall>,
        _ctx: &mut LoopContext<'_, M>,
    ) -> Result<Option<LoopOutcome>, savant_core::error::SavantError> {
        Ok(None)
    }
}

pub struct HeartbeatDelegate;
#[async_trait::async_trait]
impl<M: MemoryBackend> LoopDelegate<M> for HeartbeatDelegate {
    async fn check_signals(&self) -> LoopSignal {
        LoopSignal::Continue
    }
    async fn before_llm_call(&self, _ctx: &mut LoopContext<'_, M>) -> Option<LoopOutcome> {
        None
    }
    async fn call_llm(
        &self,
        _ctx: &mut LoopContext<'_, M>,
    ) -> Result<ChatResponse, savant_core::error::SavantError> {
        Ok(ChatResponse {
            content: String::new(),
            tool_calls: vec![],
        })
    }
    async fn handle_text_response(&self, _text: &str, _ctx: &mut LoopContext<'_, M>) -> TextAction {
        TextAction::ParseActions
    }
    async fn execute_tool_calls(
        &self,
        _calls: Vec<savant_core::types::ProviderToolCall>,
        _ctx: &mut LoopContext<'_, M>,
    ) -> Result<Option<LoopOutcome>, savant_core::error::SavantError> {
        Ok(None)
    }
}

pub struct SpeculativeDelegate;
#[async_trait::async_trait]
impl<M: MemoryBackend> LoopDelegate<M> for SpeculativeDelegate {
    async fn check_signals(&self) -> LoopSignal {
        LoopSignal::Continue
    }
    async fn before_llm_call(&self, _ctx: &mut LoopContext<'_, M>) -> Option<LoopOutcome> {
        None
    }
    async fn call_llm(
        &self,
        _ctx: &mut LoopContext<'_, M>,
    ) -> Result<ChatResponse, savant_core::error::SavantError> {
        Ok(ChatResponse {
            content: String::new(),
            tool_calls: vec![],
        })
    }
    async fn handle_text_response(&self, _text: &str, _ctx: &mut LoopContext<'_, M>) -> TextAction {
        TextAction::ParseActions
    }
    async fn execute_tool_calls(
        &self,
        _calls: Vec<savant_core::types::ProviderToolCall>,
        _ctx: &mut LoopContext<'_, M>,
    ) -> Result<Option<LoopOutcome>, savant_core::error::SavantError> {
        Ok(None)
    }
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
    pub(crate) max_tool_iterations: usize,
    pub(crate) heuristic: HeuristicState,
    pub(crate) vision_service: Option<Arc<dyn VisionProvider>>,
    pub(crate) self_repair: crate::react::self_repair::SelfRepair,
}

impl<M: MemoryBackend> AgentLoop<M> {
    pub fn new(
        agent_id: String,
        provider: Box<dyn LlmProvider>,
        memory: M,
        tools: Vec<Arc<dyn Tool>>,
        identity: AgentIdentity,
        substrate_prompt: String,
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
            context: ContextAssembler::new(
                identity,
                TokenBudget::new(256000),
                skills_list,
                substrate_prompt,
            ),
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
            max_tool_iterations: 10,
            heuristic: HeuristicState::default(),
            vision_service: None,
            self_repair: crate::react::self_repair::SelfRepair::with_defaults(),
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

    pub fn with_vision(mut self, vision: Arc<dyn VisionProvider>) -> Self {
        self.vision_service = Some(vision);
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

    pub fn with_collective(mut self, blackboard: Arc<CollectiveBlackboard>, index: u8) -> Self {
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
    use async_trait::async_trait;
    use futures::stream::StreamExt;
    use futures::Stream;
    use savant_core::error::SavantError;
    use savant_core::traits::{LlmProvider, MemoryBackend, Tool, VisionProvider};
    use savant_core::types::{AgentIdentity, ChatMessage};
    use serde_json::Value;
    use std::pin::Pin;
    use tokio::sync::Mutex;
    use tokio_util::sync::CancellationToken;

    struct MockTool {
        name: String,
        executed: Arc<Mutex<u32>>,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }
        fn description(&self) -> &str {
            "Mock"
        }
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
        async fn stream_completion(
            &self,
            _messages: Vec<ChatMessage>,
            _tools: Vec<serde_json::Value>,
        ) -> Result<
            Pin<Box<dyn Stream<Item = Result<savant_core::types::ChatChunk, SavantError>> + Send>>,
            SavantError,
        > {
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
                logprob: None,
                is_telemetry: false,
                reasoning: None,
                tool_calls: None,
            };
            Ok(Box::pin(futures::stream::iter(vec![Ok(chunk)])))
        }
    }

    struct MockMemory;
    #[async_trait]
    impl MemoryBackend for MockMemory {
        async fn store(&self, _agent_id: &str, _msg: &ChatMessage) -> Result<(), SavantError> {
            Ok(())
        }
        async fn retrieve(
            &self,
            _agent_id: &str,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<ChatMessage>, SavantError> {
            Ok(vec![])
        }
        async fn consolidate(&self, _agent_id: &str) -> Result<(), SavantError> {
            Ok(())
        }
        async fn get_or_create_session(
            &self,
            _session_id: &str,
        ) -> Result<savant_core::types::SessionState, SavantError> {
            Ok(savant_core::types::SessionState {
                session_id: "mock".to_string(),
                created_at: 0,
                last_active: 0,
                turn_count: 0,
                active_turn_id: None,
                auto_approved_tools: vec![],
                denied_tools: vec![],
            })
        }
        async fn get_session(
            &self,
            _session_id: &str,
        ) -> Result<Option<savant_core::types::SessionState>, SavantError> {
            Ok(None)
        }
        async fn save_session(
            &self,
            _state: &savant_core::types::SessionState,
        ) -> Result<(), SavantError> {
            Ok(())
        }
        async fn save_turn(
            &self,
            _turn: &savant_core::types::TurnState,
        ) -> Result<(), SavantError> {
            Ok(())
        }
        async fn get_turn(
            &self,
            _session_id: &str,
            _turn_id: &str,
        ) -> Result<Option<savant_core::types::TurnState>, SavantError> {
            Ok(None)
        }
        async fn fetch_recent_turns(
            &self,
            _session_id: &str,
            _limit: usize,
        ) -> Result<Vec<savant_core::types::TurnState>, SavantError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_speculative_chaining() {
        let executed_count = Arc::new(Mutex::new(0));
        let tool1 = Arc::new(MockTool {
            name: "Tool1".into(),
            executed: executed_count.clone(),
        });
        let tool2 = Arc::new(MockTool {
            name: "Tool2".into(),
            executed: executed_count.clone(),
        });

        let provider = Box::new(MockLlm {
            responses: vec![
                "Thought: Doing two things.\nAction: Tool1[arg1]\nAction: Tool2[arg2]".to_string(),
            ],
            call_count: Arc::new(Mutex::new(0)),
        });

        let mut agent = AgentLoop::new(
            "test_agent".into(),
            provider,
            MockMemory,
            vec![tool1, tool2],
            AgentIdentity::default(),
            String::new(),
        )
        .with_hyper_causal(crate::orchestration::branching::HyperCausalEngine::new(1));

        let mut stream = agent.run("start".into(), None, CancellationToken::new());
        while let Some(res) = stream.next().await {
            if let Err(e) = res {
                panic!("Stream error: {:?}", e);
            }
        }

        let count = executed_count.lock().await;
        assert_eq!(
            *count, 2,
            "Both tools in the speculative chain should have executed"
        );
    }
}

#[cfg(test)]
mod heuristic_tests;
