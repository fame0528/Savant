# Deep Research Prompt: Savant Full System Review

> **Purpose:** Comprehensive review of the entire Savant project for improvement and optimization  
> **Model:** Gemini 3.1 Deep Research  
> **Target Length:** 3000+ lines  
> **Instructions:** Research thoroughly. Analyze every system. Identify stress points, limits, optimization opportunities. Provide architectural recommendations grounded in production systems and code-level analysis.

---

## PART 1: PROJECT OVERVIEW

### What Savant Is

Savant is a Rust-native autonomous AI agent swarm orchestrator. It is NOT a chatbot. It is NOT an agent itself. Savant is the SUBSTRATE — the house that autonomous agents inhabit. Think of it as an operating system for autonomous coding agents that execute complex tasks across days and weeks.

- **Language:** Rust (14 crates), fully async (Tokio)
- **Database:** Fjall LSM-tree (embedded, local-first, no server)
- **API:** Axum WebSocket gateway
- **Dashboard:** Next.js 16 with real-time WebSocket
- **AI Providers:** 15 providers, defaulting to OpenRouter free tier (hunter-alpha, healer-alpha, stepfun, openrouter/free)
- **Deployment:** Local-first, no cloud dependencies, runs on user's machine
- **Memory:** 1M context window support via hunter-alpha

### The Hive Mind Architecture

Savant's fundamental design principle is the HIVE MIND. This is not a collection of independent agents. All agents share a single memory substrate. If Agent A learns something, Agent F knows it too. The substrate (Savant) IS the memory. Individual agents are processing nodes, not isolated silos.

```
┌─────────────────────────────────────────────────────────┐
│                    SAVANT SUBSTRATE                      │
│                                                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  │
│  │Agent    │  │Agent    │  │Agent    │  │Agent    │  │
│  │Alpha    │  │Beta     │  │Gamma    │  │Delta    │  │
│  │(SOUL.md)│  │(SOUL.md)│  │(SOUL.md)│  │(SOUL.md)│  │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  │
│       │            │            │            │         │
│  ┌────┴────────────┴────────────┴────────────┴────┐   │
│  │              GLOBAL MEMORY                     │   │
│  │  Fjall LSM + Vector Engine + Notifications     │   │
│  │  Auto-Recall + Bi-Temporal + Daily Logs        │   │
│  │  DAG Compaction + Promotion + Entity Graph      │   │
│  └────────────────────────────────────────────────┘   │
│                                                         │
│  ┌────────────────────────────────────────────────┐   │
│  │              GATEWAY (Axum WS)                 │   │
│  │  Auth + Config + Session Lanes + Dashboard     │   │
│  └────────────────────────────────────────────────┘   │
│                                                         │
│  ┌────────────────────────────────────────────────┐   │
│  │              CHANNELS                          │   │
│  │  Discord + Telegram + WhatsApp + Matrix        │   │
│  └────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Key Philosophies

1. **Savant is not an agent.** She is the substrate. Agents are inhabitants.
2. **Hive mind by default.** Knowledge is shared globally. No publish/subscribe gates.
3. **Local-first.** No cloud dependencies. Everything runs on the user's machine.
4. **Free models only.** Never paid models. Default: hunter-alpha (1M context).
5. **AAA quality.** Every line of code passes through the Perfection Loop (5 iterations minimum).
6. **Law 11: Universal Logic.** If two functions overlap, combine them into one.
7. **1M Context Window.** The system is architected to leverage the full 1M token context of hunter-alpha for comprehensive session understanding.

---

## PART 2: CRATE ARCHITECTURE (14 Crates)

### 2.1 savant_core (Types, Config, Crypto, DB)

**Purpose:** Foundation crate. All other crates depend on this. Contains no business logic.

**File Structure:**
- `lib.rs` — Main library entry point, re-exports all public modules
- `types/mod.rs` (1049 lines) — All shared types: ChatMessage, AgentConfig, ControlFrame, SessionId, CapabilityGrants, ExecutionMode, RequestFrame, ResponseFrame, ChatChunk, AgentIdentity, AgentOutputChannel, LlmParams
- `config.rs` (359 lines) — Config struct with auto-reload from config/savant.toml, includes LLM, gateway, memory, channels sections
- `crypto.rs` (224 lines) — Ed25519 key management, OsRng, signature verification
- `bus.rs` (195 lines) — NexusBridge (event bus for cross-crate communication), event publishing and subscription
- `db.rs` (263 lines) — Storage (Fjall-backed transactional storage with dedup)
- `traits/mod.rs` (105 lines) — Tool trait, LlmProvider trait, MemoryBackend trait, ChannelAdapter trait definitions
- `utils/embeddings.rs` — EmbeddingService (FastEmbed AllMiniLML6V2, 384-dim, LRU cache)
- `utils/parsing.rs` — Regex-based parsing utilities
- `session.rs` — Session ID sanitization

**Key Types (from types/mod.rs):**

```rust
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub sender: Option<String>,
    pub timestamp: i64,
    pub session_id: Option<String>,
    pub channel: AgentOutputChannel,
    pub metadata: Option<HashMap<String, String>>,
}

pub struct AgentConfig {
    pub agent_id: String,
    pub name: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub tools: Vec<String>,
    pub avatar_url: Option<String>,
    pub personality_traits: Option<HashMap<String, f32>>,
}

pub enum RequestPayload {
    ChatMessage(ChatMessage),
    ControlFrame(ControlFrame),
    Auth(String),
}

pub struct ResponseFrame {
    pub request_id: String,
    pub payload: String,
}

pub enum ControlFrame {
    ConfigGet(String),
    ConfigSet { key: String, value: String },
    ModelsList,
    SoulManifest,
    NLCommand(String),
    Heartbeat,
    Shutdown,
}
```

**Key implementations:**

```rust
// From bus.rs - NexusBridge event system
pub struct NexusBridge {
    sender: broadcast::Sender<(String, Vec<u8>)>,
    // Event types: "chat.message", "agent.response", "config.changed", "tool.executed"
}

impl NexusBridge {
    pub fn new() -> Self;
    pub fn publish(&self, event: &str, payload: &[u8]) -> Result<(), NexusError>;
    pub fn subscribe(&self, event: &str) -> broadcast::Receiver<(String, Vec<u8>)>;
    pub async fn update_state(&self, key: String, value: String); // Global state broadcast
}
```

```rust
// From config.rs - Configuration management
pub struct Config {
    pub llm: LlmConfig,
    pub gateway: GatewayConfig,
    pub memory: MemoryConfig,
    pub channels: ChannelsConfig,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError>;
    pub fn reload(&mut self) -> Result<(), ConfigError>; // File watcher integration
}
```

**Stress points:**
1. **EmbeddingService is single-threaded** (Mutex around TextEmbedding). Bottleneck for concurrent agents. Located in `utils/embeddings.rs` - uses a single global Mutex lock for all embedding operations.
2. **Config auto-reload via file watcher** has race conditions if multiple writes happen rapidly. The `reload()` method in `config.rs` doesn't use atomic swaps.
3. **NexusBridge uses broadcast channel** - limited buffer size (default 16), can drop events if subscribers are slow.
4. **No request size limits** on ChatMessage content - potential OOM from large payloads.
5. **ChatRole enum** is limited: User, Assistant, System, Tool - no support for multi-party context.

### 2.2 savant_agent (Agents, Swarm, Providers, Tools)

**Purpose:** The agent runtime. Manages agent lifecycles, swarm orchestration, LLM providers, tool execution.

**File Structure:**
- `lib.rs` — Main library entry point
- `manager.rs` (35 lines) — AgentManager: discovers agents from workspaces/
- `swarm.rs` (596 lines) — SwarmController: orchestrates agents, discovers tools, starts heartbeat loops
- `context.rs` (159 lines) — ContextAssembler: builds system prompt, assembles messages with token budget
- `react/mod.rs` (246 lines) — AgentLoop: the ReAct loop with HeuristicState for retry/failure tracking
- `react/events.rs` — AgentEvent types
- `react/reactor.rs` — Reactor pattern implementation
- `react/stream.rs` — Async stream utilities
- `budget.rs` — TokenBudget: tier allocation (system 20%, recent 50%, semantic 20%, old 10%)
- `providers/mod.rs` (1056 lines) — 15 LLM providers with unified interface
- `providers/mgmt.rs` — Provider management utilities
- `pulse/heartbeat.rs` — HeartbeatPulse: non-blocking loop multiplexing
- `proactive/` — ProactivePartner: filesystem monitoring, git status
- `ensemble/mod.rs` — EnsembleRouter: multi-model dispatch
- `nlp/commands.rs` — Natural language command parser
- `free_model_router.rs` — Model selection cascade
- `plugins/wasm_host.rs` — WASM tool execution
- `orchestration/branching/` — HyperCausalEngine for speculative execution

**Key implementations:**

```rust
// From swarm.rs - SwarmController
pub struct SwarmController {
    pub agents: Vec<AgentConfig>,
    pub memory: Arc<MemoryEngine>,
    pub nexus: Arc<NexusBridge>,
    pub providers: HashMap<String, Box<dyn LlmProvider>>,
    pub skills: Vec<Arc<dyn Tool>>,
    pub blackboard: Option<Arc<SwarmBlackboard>>,
    pub echo_registry: Option<Arc<HotSwappableRegistry>>,
    pub config: Config,
    // ~25 fields total - needs decomposition
}

impl SwarmController {
    pub fn new(config: Config) -> Result<Self, SavantError>;
    pub fn ignite(&self) -> Result<(), SavantError>; // Start all agents
    pub fn discover_tools(&self) -> Result<Vec<Arc<dyn Tool>>, SavantError>;
    pub fn shutdown(&self) -> Result<(), SavantError>;
}
```

```rust
// From context.rs - ContextAssembler
pub struct ContextAssembler {
    pub identity: AgentIdentity,
    pub budget: TokenBudget,
    pub skills_list: Option<String>,
    pub soul_md: Option<String>,
    pub context_cache: Option<String>,
}

impl ContextAssembler {
    pub fn new(identity: AgentIdentity, budget: TokenBudget, skills_list: Option<String>) -> Self;
    pub fn build_messages(&self, history: &[ChatMessage], query: &str) -> Vec<ChatMessage>;
    pub fn inject_context_cache(&mut self, block: ContextCacheBlock);
    pub fn assemble_system_prompt(&self) -> String;
}
```

```rust
// From budget.rs - TokenBudget
pub struct TokenBudget {
    pub system_pct: u8,   // 20%
    pub recent_pct: u8,   // 50%
    pub semantic_pct: u8, // 20%
    pub old_pct: u8,      // 10%
    pub max_tokens: usize, // 256000 default for 1M context
}

impl TokenBudget {
    pub fn new(max_tokens: usize) -> Self;
    pub fn allocate(&self, messages: &[ChatMessage]) -> Vec<ChatMessage>;
    pub fn truncate(&self, messages: &mut Vec<ChatMessage>);
}
```

**LLM Providers (from providers/mod.rs):**

```rust
// 15 providers implemented:
pub struct OpenAiProvider { client, api_key, model, agent_id, agent_name, llm_params }
pub struct OpenRouterProvider { ... }  // Default: hunter-alpha
pub struct AnthropicProvider { ... }   // Claude integration
pub struct OllamaProvider { ... }     // Local models
pub struct GroqProvider { ... }       // Fast inference
pub struct GoogleProvider { ... }      // Gemini
pub struct MistralProvider { ... }
pub struct TogetherProvider { ... }
pub struct DeepseekProvider { ... }
pub struct CohereProvider { ... }      // Command R
pub struct AzureProvider { ... }       // Enterprise
pub struct XaiProvider { ... }         // Grok
pub struct FireworksProvider { ... }
pub struct NovitaProvider { ... }
pub struct RetryProvider { ... }        // Decorator with retry logic

// RetryProvider implementation (lines 997-1056)
impl LlmProvider for RetryProvider {
    async fn stream_completion(&self, messages: Vec<ChatMessage>) -> 
        Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError> {
        // Retries on 429, 500-504, network errors
        // Exponential backoff: 500ms * attempt
        // Max retries configurable
    }
}
```

**Stress points:**
1. **SwarmController holds too many fields** (~25 fields). Documented need for decomposition into focused subsystems.
2. **HeartbeatPulse does synchronous filesystem operations** in async context - blocks the runtime.
3. **AgentLoop has no timeout enforcement** for long-running tool calls.
4. **Token budget is fixed percentages** - doesn't adapt to task type or query complexity.
5. **Free model router cascade** is hardcoded: hunter-alpha → healer-alpha → stepfun → openrouter/free. No quality-based selection.
6. **ContextAssembler rebuilds system prompt** on every call - no caching.

### 2.3 savant_memory (Fjall, Vectors, Semantic Search, Auto-Recall, Bi-Temporal)

**Purpose:** The memory substrate. Global storage shared across all agents.

**File Structure:**
- `lib.rs` — Main library entry point
- `engine.rs` (532 lines) — MemoryEngine: unified facade
- `lsm_engine.rs` (677 lines) — LsmStorageEngine: Fjall 3.0 OptimisticTxDatabase
- `vector_engine.rs` (826 lines) — SemanticVectorEngine: ruvector-core HNSW
- `async_backend.rs` — AsyncMemoryBackend: async wrapper
- `models.rs` (708 lines) — All data models
- `daily_log.rs` — Daily operational logs
- `notifications.rs` — NotificationChannel
- `promotion.rs` — PromotionEngine: OCEAN trait-based scoring
- `entities.rs` — EntityExtractor
- `error.rs` — MemoryError with 13 variants
- `safety.rs` — Kani verification harnesses

**Key implementations:**

```rust
// From lsm_engine.rs - Fjall LSM storage
pub struct LsmStorageEngine {
    db: OptimisticTxDatabase,
    transcript_ks: OptimisticTxKeyspace,
    _metadata_ks: Option<OptimisticTxKeyspace>,
    temporal_ks: OptimisticTxKeyspace,
    dag_ks: OptimisticTxKeyspace,
}

impl LsmStorageEngine {
    pub fn new(storage_path: &Path, config: LsmConfig) -> Result<Arc<Self>, MemoryError>;
    pub fn append_message(&self, session_id: &str, message: &AgentMessage) -> Result<(), MemoryError>;
    pub fn fetch_session_tail(&self, session_id: &str, limit: usize) -> Vec<AgentMessage>;
    pub fn insert_metadata(&self, id: u64, entry: &MemoryEntry) -> Result<(), MemoryError>;
    pub fn remove_metadata(&self, id: u64) -> Result<(), MemoryError>;
}

// Keyspaces:
// - "transcripts": conversation messages (session_id + timestamp + message_id -> AgentMessage)
// - "metadata": MemoryEntry objects
// - "temporal_metadata": TemporalMetadata (valid_from, valid_to)
// - "dag_nodes": DagNode for compaction
```

```rust
// From vector_engine.rs - SIMD vector search
pub struct SemanticVectorEngine {
    db: Arc<VectorDB>,
    _quantizer: Option<BinaryQuantized>,
    config: VectorConfig,
    entries: Arc<Mutex<Vec<VectorEntry>>>,
    persist_path: Option<PathBuf>,
}

pub struct VectorConfig {
    pub dimensions: usize,           // 384 default
    pub hnsw_m: usize,               // 16
    pub hnsw_ef_construction: usize, // 200
    pub hnsw_ef_search: usize,       // 50
    pub use_quantization: bool,      // true (32x compression)
}

impl SemanticVectorEngine {
    pub fn new(path: &Path, config: VectorConfig) -> Result<Arc<Self>, MemoryError>;
    pub fn index_memory(&self, memory_id: &str, embedding: &[f32]) -> Result<(), MemoryError>;
    pub fn recall(&self, query_embedding: &[f32], top_k: usize, options: Option<SearchOptions>) 
        -> Result<Vec<SearchResult>, MemoryError>;
    pub fn save_to_path(&self, path: &Path) -> Result<(), MemoryError>;
    pub fn load_from_path(path: &Path, config: VectorConfig) -> Result<Arc<Self>, MemoryError>;
    pub fn simd_supported() -> bool; // AVX2/AVX-512/NEON detection
}
```

```rust
// From models.rs - Core data structures
pub struct AgentMessage {
    pub id: String,
    pub role: ChatRole,
    pub content: String,
    pub sender: Option<String>,
    pub timestamp: Timestamp,
    pub session_id: String,
    pub tool_calls: Vec<ToolCallRef>,
    pub tool_results: Vec<ToolResultRef>,
    pub importance: u8,  // 0-10 scale for auto-recall threshold
}

pub struct MemoryEntry {
    pub id: u64,
    pub content: String,
    pub embedding: Vec<f32>,
    pub importance: u8,
    pub created_at: i64,
    pub accessed_at: i64,
    pub tags: Vec<String>,
    pub entity_type: Option<String>,
}

pub struct TemporalMetadata {
    pub valid_from: i64,
    pub valid_to: Option<i64>,
    pub assert_time: i64,
}

pub struct ContextCacheBlock {
    pub memories: Vec<MemoryEntry>,
    pub relevance_scores: Vec<f32>,
    pub generated_at: i64,
}

pub struct AutoRecallConfig {
    pub enabled: bool,
    pub top_k: usize,           // 5
    pub similarity_threshold: f32, // 0.3
    pub importance_threshold: u8, // 7
}
```

```rust
// From promotion.rs - OCEAN-based promotion
pub struct PromotionEngine {
    ocean_weights: OceanWeights,
    decay_factor: f32,
    min_importance: u8,
}

impl PromotionEngine {
    pub fn score(&self, entry: &MemoryEntry) -> f32;
    pub fn promote(&self, memory: &mut MemoryEntry);
    pub fn demote(&self, memory: &mut MemoryEntry);
    pub fn should_promote(&self, entry: &MemoryEntry) -> bool;
}

pub struct OceanWeights {
    pub openness: f32,
    pub conscientiousness: f32,
    pub extraversion: f32,
    pub agreeableness: f32,
    pub neuroticism: f32,
}
```

```rust
// From entities.rs - Rule-based entity extraction
pub struct EntityExtractor {
    patterns: HashMap<String, Regex>,
}

impl EntityExtractor {
    pub fn new() -> Self;
    pub fn extract(&self, text: &str) -> Vec<Entity>;
    pub fn extract_project(&self, text: &str) -> Option<String>;
    pub fn extract_service(&self, text: &str) -> Option<String>;
    pub fn extract_credential(&self, text: &str) -> Option<String>;
    pub fn extract_file(&self, text: &str) -> Option<String>;
    pub fn extract_config(&self, text: &str) -> Option<String>;
}
```

**Key data flow:**
```
store() → AgentMessage → append_message → embed → index_memory → notify (if importance >= 7)
retrieve() → embed query → semantic_search → session tail → merge → filter temporal
auto_recall() → embed last 3 user messages → semantic_search → ContextCacheBlock
promotion() → OCEAN scoring → demote/promote based on access patterns
```

**Stress points:**
1. **fetch_message_by_id() is O(N) scan** - no reverse index. Iterates entire keyspace prefix.
2. **atomic_compact() is DESTRUCTIVE** - deletes all messages before inserting compacted batch. DAG nodes reference messages that could be lost.
3. **Vector engine uses single global index** - no per-agent or per-session isolation.
4. **EntityExtractor is rule-based** - regex patterns have limited accuracy. No NER integration.
5. **Fjall grows unbounded** - no background compaction schedule.
6. **PromotionEngine has no background worker** - memories never promoted automatically.
7. **309GB protection in fetch_session_tail** - validates raw byte length before deserialization to prevent OOM.

### 2.4 savant_gateway (Axum WebSocket, Auth, Config, Lanes)

**Purpose:** The gateway server. All external communication goes through here.

**File Structure:**
- `lib.rs` — Main library entry point
- `server.rs` (405 lines) — GatewayState, Axum router, WebSocket handler
- `auth/mod.rs` — Ed25519 signature verification, timestamp drift validation
- `handlers/mod.rs` — Request dispatch
- `lanes.rs` (118 lines) — SessionLane with semaphore backpressure

**Key implementations:**

```rust
// From server.rs - WebSocket gateway
pub struct GatewayState {
    pub nexus: Arc<NexusBridge>,
    pub config: Config,
    pub sessions: Arc<Mutex<HashMap<SessionId, SessionLane>>>,
    pub avatar_cache: Mutex<HashMap<String, String>>,
}

impl GatewayState {
    pub fn new(config: Config, nexus: Arc<NexusBridge>) -> Self;
    pub async fn handle_websocket(&self, ws: WebSocketUpgrade) -> impl IntoResponse;
}
```

```rust
// From lanes.rs - SessionLane with backpressure
pub struct SessionLane {
    pub tx: mpsc::Sender<RequestFrame>,
    pub response_tx: mpsc::Sender<ResponseFrame>,
}

impl SessionLane {
    pub fn new(capacity: usize, max_concurrent: usize) -> (Self, Receivers, Arc<Semaphore>);
    pub fn spawn_consumer(
        rx: mpsc::Receiver<RequestFrame>,
        response_tx: mpsc::Sender<ResponseFrame>,
        concurrency_limit: Arc<Semaphore>,
        nexus: Arc<NexusBridge>,
    );
    // Backpressure: 30s timeout on concurrency acquisition
}

// Directive processing:
// - Max length: 2048 chars
// - No control characters (except \n, \r, \t)
// - Global directive broadcast via NexusBridge
```

```rust
// From auth/mod.rs - Authentication
pub fn verify_signature(
    message: &str,
    signature: &[u8],
    public_key: &[u8; 32],
) -> Result<bool, AuthError>;

pub fn validate_timestamp(drift_secs: i64) -> Result<(), AuthError>;
// Max drift: 300 seconds (5 minutes)
```

**Stress points:**
1. **Single WebSocket endpoint per session** - no multiplexing.
2. **No connection rate limiting per IP** - planned but not implemented.
3. **CORS allows all origins** - security gap.
4. **ConfigSet overwrites one key at a time** - no batch config update.
5. **Avatar cache is in-memory only** - no disk persistence, lost on restart.
6. **SessionLane capacity is 100, max_concurrent 12** - bottlenecks for high-throughput.

### 2.5 savant_skills (Docker, WASM, Lambda, Native, Security, Parser)

**Purpose:** Skill execution system. Skills are plugins that extend agent capabilities.

**File Structure:**
- `lib.rs` — Main library entry point
- `parser.rs` (854 lines) — SkillRegistry, SecurityGateResult
- `security.rs` (1777 lines) — SecurityScanner with 20+ regex patterns
- `sandbox/mod.rs` (71 lines) — SandboxDispatcher, ToolExecutor trait
- `sandbox/wasm.rs` — WassetteExecutor
- `sandbox/native.rs` — LegacyNativeExecutor with Landlock
- `docker.rs` — DockerSkillExecutor
- `lambda.rs` — LambdaSkillExecutor
- `wasm/mod.rs` — WasmSkillExecutor
- `clawhub.rs` — ClawHubClient
- `hot_reload.rs` — SkillHotReload

**Key implementations:**

```rust
// From parser.rs - Skill parsing and security gate
pub struct SkillRegistry {
    skills: HashMap<String, SkillManifest>,
    security_gate: SecurityScanner,
}

pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub execution_mode: ExecutionMode,
    pub capabilities: CapabilityGrants,
    pub security_gate: SecurityGateResult,
}

pub enum SecurityGateResult {
    AutoApproved { scan_result: SecurityScanResult },
    PendingApproval { scan_result: SecurityScanResult, clicks_completed: u32, clicks_required: u32 },
    UserApproved { scan_result: SecurityScanResult, approved_at: i64, clicks_completed: u32 },
    UserRejected { scan_result: SecurityScanResult, rejected_at: i64 },
}

// Click requirements:
// - Clean/Low: 0 clicks (auto-proceed)
// - Medium: 1 click
// - High: 2 clicks (double-confirm)
// - Critical: 3 clicks (triple-confirm)
```

```rust
// From security.rs - Threat detection
pub struct SecurityScanner {
    patterns: Vec<SecurityPattern>,
    blocklist: HashSet<String>,
    malicious_names: HashSet<String>,
    malicious_domains: HashSet<String>,
}

// 20+ security patterns:
// 1. Credential theft (API keys, passwords, tokens)
// 2. Reverse shell indicators
// 3. Cryptomining patterns
// 4. Keylogger patterns
// 5. Clipboard hijacking
// 6. Persistent state injection
// 7. Lateral movement attempts
// 8. Dependency confusion
// 9. Typosquatting
// 10. Time-bomb detection
// ... and more

pub enum RiskLevel {
    Clean,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn clicks_required(&self) -> u32;
    pub fn icon(&self) -> &str;
    pub fn color(&self) -> &str;
    pub fn bg_color(&self) -> &str;
    pub fn warning_message(&self) -> &str;
}
```

```rust
// From sandbox/mod.rs - Execution routing
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, args: Value) -> Result<String, SavantError>;
}

pub struct SandboxDispatcher;

impl SandboxDispatcher {
    pub fn create_executor(
        mode: &ExecutionMode,
        workspace_dir: PathBuf,
        capabilities: CapabilityGrants,
    ) -> Box<dyn ToolExecutor>;
}

// ExecutionMode variants:
// - WasmComponent(url) -> wasmtime-based WASM
// - LegacyNative(script) -> Landlock-sandboxed shell
// - DockerContainer(image) -> bollard-based Docker

// WASM fuel limit: 50M instructions
// Landlock: filesystem restrictions
// Docker: image-based isolation
```

**Stress points:**
1. **WASM executor loads from OCI registries** - network dependency, can fail.
2. **Docker executor has no seccomp profiles** - broad syscall access.
3. **Lambda executor uses simplified auth** - no proper AWS SDK signing.
4. **Security scanner uses regex** - limited accuracy, no semantic analysis.
5. **Hot-reload polls every 2 seconds** - not instant detection.
6. **No skill versioning** - can't roll back to previous version.
7. **Threat intelligence sync** - MalwareBazaar + URLhaus, but no local caching.

### 2.6 savant_mcp (Model Context Protocol Server and Client)

**Purpose:** MCP server for tool exposure and MCP client for external tool discovery.

**File Structure:**
- `lib.rs` — Main library entry point
- `server.rs` — MCP WebSocket server
- `client.rs` — McpClient, McpRemoteTool
- `circuit.rs` — CircuitBreaker

**Stress points:**
1. **MCP client creates new WebSocket** - no connection pooling.
2. **MCP server rate limiting per-connection** - no global rate limiting.
3. **McpRemoteTool doesn't cache results** - repeated calls hit server every time.

### 2.7 savant_channels (Discord, Telegram, WhatsApp, Matrix)

**Purpose:** Multi-platform communication adapters.

**File Structure:**
- `lib.rs` — Main library entry point
- `discord.rs` — serenity-based adapter
- `telegram.rs` — teloxide-based adapter
- `whatsapp.rs` — WhatsApp Web sidecar

**Stress points:**
1. **Discord adapter spawns separate Tokio runtime** - isolation but overhead.
2. **WhatsApp sidecar has no health monitoring** - can fail silently.
3. **No channel-level message queuing** - messages lost during reconnection.

### 2.8 savant_cognitive (DSP Predictor, Synthesis, Planning)

**Purpose:** Cognitive planning and complexity estimation.

**File Structure:**
- `lib.rs` — Main library entry point
- `predictor.rs` — DspPredictor: asymmetric expectile loss
- `synthesis.rs` — Goal decomposition, plan synthesis
- `persistence.rs` — Predictor state serialization

**Stress points:**
1. **DspPredictor is single-threaded** - no concurrent access support.
2. **Goal decomposition uses hardcoded conjunction list** - limited to English.
3. **No undo/redo for trajectory refinement.**

### 2.9 savant_echo (Circuit Breaker, Hot-Swap Compiler)

**Purpose:** Self-healing and hot-swap capability.

**File Structure:**
- `lib.rs` — Main library entry point
- `circuit_breaker.rs` (735 lines) — ComponentMetrics, CircuitState
- `registry.rs` — HotSwappableRegistry
- `compiler.rs` — EchoCompiler

**Key implementations:**

```rust
// From circuit_breaker.rs - Statistical circuit breaker
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed = 0,   // Normal operation
    Open = 1,      // Blocked
    HalfOpen = 2,  // Testing recovery
}

pub struct ComponentMetrics {
    total_invocations: AtomicU64,
    failed_invocations: AtomicU64,
    error_threshold: f64,
    min_sample_size: u64,
    state: AtomicU8,
    opened_at: AtomicU64,
    reset_duration_secs: u64,
    consecutive_successes: AtomicU64,
    success_threshold: u64,
    reset_count: AtomicU64,
    trip_count: AtomicU64,
}

impl ComponentMetrics {
    pub fn new(error_threshold: f64, min_sample_size: u64) -> Self;
    pub fn record_outcome(&self, success: bool) -> bool; // Returns true if should block
    pub fn state(&self) -> CircuitState;
    pub fn reset(&self);
    pub fn error_rate(&self) -> f64;
    // Time-based reset: 300 seconds default
    // Success threshold: 5 consecutive successes
}
```

**Stress points:**
1. **No hot-swap compiler actually implemented** - only circuit breaker exists.
2. **ComponentMetrics has no persistence** - state lost on restart.
3. **Error threshold is static** - doesn't adapt to component behavior.

### 2.10 savant_security (Token Minting, Attestation)

**Purpose:** Security infrastructure for token management and attestation.

**File Structure:**
- `lib.rs` — Main library entry point
- `enclave.rs` — CapabilityPayload, token minting
- `attestation.rs` — TPM and WASM attestation
- `proofs.rs` — Kani proof harnesses

**Stress points:**
1. **Token minting uses Ed25519 only** - not quantum-safe (Dilithium2 declared but not used).
2. **Attestation verification is stubbed** - returns Ok without actual verification.
3. **No token rotation mechanism** - keys never expire.

### 2.11 savant_canvas (A2UI Rendering, LCS Diff)

**Purpose:** Dashboard rendering engine and diff computation.

**File Structure:**
- `lib.rs` — Main library entry point
- `a2ui.rs` — CanvasManager
- `diff.rs` — LCS-based array diff

**Stress points:**
1. **Canvas manager is in-memory only** - no persistence.
2. **Diff computation doesn't handle nested objects.**

### 2.12 savant_ipc (Blackboard, Collective Voting)

**Purpose:** Inter-process communication via iceoryx2 zero-copy shared memory.

**File Structure:**
- `lib.rs` — Main library entry point
- `blackboard.rs` — SwarmSharedContext, DelegationBloomFilter
- `collective.rs` — AgentEntry, GlobalState, CollectiveBlackboard

**Stress points:**
1. **iceoryx2 is a system dependency** - requires manual installation.
2. **Bloom filter has fixed 256-bit capacity** - too many agents causes false positives.
3. **No persistent IPC state** - lost on restart.

### 2.13 savant_panopticon (Observability)

**Purpose:** Observability and event recording.

**File Structure:**
- `lib.rs` — Main library entry point
- `replay.rs` — ReplayRecorder

**Stress points:**
1. **Replay recorder is in-memory only** - no disk persistence.
2. **No integration with external tools** - Prometheus, Grafana missing.

### 2.14 savant_cli (Command-Line Interface)

**Purpose:** CLI entry point for the Savant system.

**File Structure:**
- `main.rs` — CLI entry point

**Subcommands:**
- `start` — Start the Savant server
- `test-skill` — Test a skill
- `backup` — Backup memory
- `restore` — Restore from backup
- `list-agents` — List available agents
- `status` — Show system status
- `--keygen` — Generate Ed25519 keys

**Stress points:**
1. **No daemon mode** - runs in foreground only.
2. **No log rotation** - logs grow unbounded.

---

## PART 3: CROSS-SYSTEM CONNECTIONS

### 3.1 Data Flow: User Message → Agent Response

```
User types in Dashboard
    ↓
WebSocket message to gateway /ws
    ↓
Gateway: auth verify → SessionLane::tx.send(frame)
    ↓
SessionLane consumer: deserialize RequestFrame
    ↓
ControlFrame dispatch or Chat message
    ↓
NexusBridge.publish("chat.message", payload)
    ↓
HeartbeatPulse receives via chat_rx
    ↓
AgentLoop::run(): react loop
    ↓
ContextAssembler::build_messages():
    ├── assemble_system_prompt() (SOUL.md + instructions + tools)
    ├── auto_recall() → EmbeddingService → semantic_search → ContextCacheBlock
    ├── inject <context_cache> block
    └── append conversation history (token budget enforced)
    ↓
LlmProvider::complete(): send to hunter-alpha via OpenRouter
    ↓
Parse response: tool calls or text
    ↓
If tool call → SandboxDispatcher::create_executor() → execute in sandbox
    ↓
If text → ChatMessage{role: Assistant} → NexusBridge.publish
    ↓
SessionLane::response_tx.send(ResponseFrame)
    ↓
Dashboard WebSocket receives → renders
```

### 3.2 Data Flow: Agent Discovery → Swarm Ignition

```
AgentManager::discover_agents() scans workspaces/agents/
    ↓
For each SOUL.md found → create AgentConfig
    ↓
SwarmController::new() initializes:
├── MemoryEngine (Fjall + VectorEngine)
├── Tool discovery (skills/ directory)
├── NexusBridge (event bus)
├── SwarmBlackboard (iceoryx2 shared memory)
├── 15 LLM providers
└── EchoCompiler + HotSwappableRegistry
    ↓
SwarmController::ignite():
For each agent:
├── Create AgentLoop with provider + memory + tools
├── Create HeartbeatPulse
├── Start tokio::spawn(heartbeat.start(agent_loop))
└── Publish to NexusBridge
    ↓
All agents running, listening on NexusBridge
```

### 3.3 Data Flow: Memory Store → Auto-Recall

```
Agent uses tool → tool result received
    ↓
AsyncMemoryBackend::store(agent_id, &ChatMessage)
    ↓
AgentMessage::from_chat() → convert format
    ↓
MemoryEngine::append_message() → write to Fjall LSM
    ↓
EmbeddingService::embed(content) → 384-dim vector
    ↓
MemoryEngine::index_memory(entry) → store in VectorEngine + metadata
    ↓
If importance >= 7 → NotificationChannel::notify()
    ↓
(auto later)
User asks new question
    ↓
AsyncMemoryBackend::auto_recall(agent_id, query, config)
    ↓
EmbeddingService::embed(last 3 user messages)
    ↓
MemoryEngine::semantic_search(embedding, 5)
    ↓
Filter by similarity_threshold (0.3)
    ↓
ContextCacheBlock → injected as <context_cache> in system prompt
```

### 3.4 Data Flow: Skill Execution

```
Agent decides to use skill
    ↓
SkillRegistry::get_tool(name) → Tool instance
    ↓
SkillManifest::execution_mode determines sandbox
    ↓
SandboxDispatcher::create_executor(mode, workspace, capabilities)
    ↓
ToolExecutor::execute(args)
    ↓
If WASM → wasmtime component load from OCI
    ↓
If Native → Landlock sandbox + script execution
    ↓
If Docker → bollard container run
    ↓
Result returned to agent
    ↓
MemoryEngine::store(tool_result)
```

---

## PART 4: KNOWN STRESS POINTS AND LIMITS

### 4.1 Performance Limits

| Component | Limit | Impact |
|-----------|-------|--------|
| EmbeddingService | Single-threaded (Mutex) | Concurrent agents block on embedding |
| VectorEngine | In-memory HNSW, no disk paging | Memory grows with vector count |
| Fjall LSM | No background compaction | Storage grows unbounded |
| fetch_message_by_id() | O(N) scan | Slow for large sessions |
| SessionLane | 100 capacity, 12 concurrent | Bottleneck for high-throughput |
| WASM fuel | 50M instructions | Complex skills may timeout |
| Config auto-reload | File watcher race conditions | Rapid writes cause conflicts |
| TokenBudget | 256K tokens max | Cannot use full 1M context |
| Auto-Recall | top_k=5, threshold=0.3 | May miss relevant context |
| HeartbeatPulse | Sync filesystem in async | Blocks runtime |
| DspPredictor | Single-threaded | No concurrent estimation |
| Vector persist | 10M vectors max | Limited scale |

### 4.2 Security Limits

| Component | Limit | Impact |
|-----------|-------|--------|
| CORS | Allows all origins | Any website can connect |
| Request size | No limits | Large payloads cause OOM |
| Docker sandbox | No seccomp profiles | Broad syscall access |
| Token minting | Ed25519 only | Not quantum-safe |
| Attestation | Stubbed (returns Ok) | No actual verification |
| Capability grants | Manifest-only | Not enforced at runtime |
| IP rate limiting | Not implemented | DoS vulnerability |
| Session isolation | None | Cross-session leakage possible |

### 4.3 Architectural Limits

| Component | Limit | Impact |
|-----------|-------|--------|
| Hive mind | No per-agent isolation | Personality contamination |
| DAG compaction | Destructive | Original messages can be lost |
| Entity extraction | Rule-based only | Low accuracy |
| Daily logs | No rotation | Logs grow forever |
| Promotion engine | No background worker | Memories never promoted |
| Notifications | No persistent queue | Lost if no subscribers |
| Config reload | No atomic swap | Race conditions |
| WebSocket | Single endpoint | No multiplexing |
| Bloom filter | 256-bit fixed | False positives at scale |
| IPC state | Not persistent | Lost on restart |

### 4.4 Scalability Limits

| Metric | Current Limit | Bottleneck |
|--------|--------------|------------|
| Concurrent agents | ~50 | HeartbeatPulse overhead |
| Vector index | ~100K vectors | HNSW memory |
| Session messages | ~10K | Fjall scan |
| Skills | ~100 | Registry linear scan |
| WebSocket connections | 1000 | Axum defaults |
| Dashboard pages | 6 | Single page.tsx |
| MCP servers | ~10 | No connection pool |
| Channels | 4 platforms | Per-platform adapters |

---

## PART 5: RESEARCH QUESTIONS

### On Architecture

1. **Hive Mind vs Agent Isolation:** The hive mind model means all agents share memory. What are the failure modes? Can Agent A's personality contaminate Agent B's context? How do production multi-agent systems handle this?

2. **Substrate vs Agent Boundary:** Savant is the substrate, agents are inhabitants. Where should the boundary be? Should agents have their own memory partitions, or should everything be global?

3. **Swarm Size:** What's the optimal number of concurrent agents? How does performance degrade as the swarm grows? Is there a sweet spot?

4. **Crate Decomposition:** The SwarmController has ~25 fields. What's the best way to decompose a monolithic controller into focused subsystems?

5. **1M Context Window Usage:** With hunter-alpha's 1M context, how should token budget allocation change? Current 256K limit seems wasteful.

### On Memory

6. **EmbeddingService Scaling:** Single-threaded embedding is a bottleneck. What are the options for parallel embedding? Batch processing? Multiple FastEmbed instances?

7. **Fjall Compaction:** No background compaction causes storage growth. What's the best compaction strategy for agent memory? Size-based? Time-based? Access-frequency-based?

8. **Vector Index Persistence:** HNSW index is in-memory. What's the best approach for persistent vector indexes in a local-first system?

9. **Auto-Recall Accuracy:** The current auto-recall uses simple cosine similarity. How do production systems improve retrieval accuracy? Re-ranking? Hybrid search? Learned embeddings?

10. **Bi-Temporal Complexity:** Is full bi-temporal tracking (valid_from + valid_to) worth the complexity, or is a simpler version chain sufficient?

11. **Entity Resolution:** Rule-based entity extraction has limited accuracy. Is NER (like gline-rs) worth the dependency weight? Or can the LLM itself be used for entity extraction during write?

12. **DAG Compaction Safety:** atomic_compact() is destructive. How can we make it non-destructive while still achieving compaction goals?

### On Skills

13. **WASM vs Docker:** When should a skill use WASM vs Docker? What are the trade-offs in terms of performance, security, and capability?

14. **Skill Security:** The security scanner uses regex patterns. Is that sufficient for production? What are the false positive/negative rates?

15. **Hot-Reload Latency:** 2-second polling is not instant. What's the best approach for instant skill reload? inotify? WebSocket notification?

16. **Skill Versioning:** How should skill versions be managed? Can we roll back to previous versions?

### On Gateway

17. **WebSocket Scaling:** Single WebSocket per session. How do you scale to thousands of concurrent connections?

18. **Config Management:** Dashboard can change config via WebSocket. What's the best way to handle config conflicts when multiple clients change settings simultaneously?

19. **Session Isolation:** SessionLane provides per-session isolation. But what happens when one session hogs resources?

20. **CORS Security:** Allowing all origins is a security gap. What's the recommended CORS policy for a local-first WebSocket server?

### On Security

21. **Token Rotation:** OpenRouter Management API creates per-agent keys. How do you handle key rotation without downtime?

22. **WASM Sandboxing:** wasmtime provides fuel limits and memory caps. Is that sufficient? What about network access, filesystem, or other side channels?

23. **Quantum-Safe Cryptography:** Ed25519 is not quantum-safe. How should Dilithium2 integration work? What are the performance implications?

24. **Attestation:** The attestation system is stubbed. How should TPM and WASM attestation actually work?

### On Cognitive

25. **Plan Quality:** The synthesis engine generates plans based on keyword matching. How do production planning systems improve plan quality? MCTS? LLM-based planning?

26. **Context Window Management:** Token budget allocation is fixed (20/50/20/10). Should it be dynamic based on task type?

27. **Multi-Model Ensemble:** Best-of-N selection uses heuristic scoring. How do production ensemble systems select the best response? Learned quality models?

28. **DSP Predictor:** Asymmetric expectile regression is complex. What are the optimal tau and beta values for agent task estimation?

### On Channels

29. **Message Queuing:** No channel-level message queuing means messages can be lost during reconnection. What's the best approach for reliable message delivery?

30. **Multi-Platform Consistency:** Different channels have different message formats. How do you maintain consistent agent personality across channels?

31. **WhatsApp Sidecar:** The sidecar process has no health monitoring. What's the best approach for sidecar process management?

### On Echo/Hot-Swap

32. **Circuit Breaker Metrics:** ComponentMetrics tracks failures but doesn't adapt thresholds. Should error_threshold be dynamic?

33. **Hot-Swap Compiler:** The echo crate name suggests compilation but no compiler exists. What's the vision for hot-swap?

34. **Persistence:** Circuit breaker state is lost on restart. Should it be persisted?

### On IPC

35. **Iceoryx2 Dependency:** iceoryx2 requires manual installation. Should we bundle or use an alternative?

36. **Bloom Filter Capacity:** 256-bit is fixed. What's the right size for large swarms?

### On Observability

37. **Replay Recorder:** In-memory only. Should we persist events to disk?

38. **External Integration:** No Prometheus/Grafana. Should we add metrics endpoints?

### On CLI

39. **Daemon Mode:** No daemon mode. Should we add background running?

40. **Log Rotation:** Logs grow unbounded. What's the right rotation strategy?

---

## PART 6: EASTER EGG SYSTEMS

Current easter eggs are designed but not yet implemented:

| Egg | Trigger | Effect |
|-----|---------|--------|
| The Oracle | Dashboard idle 5 min | AI-generated prediction |
| Konami Code | ↑↑↓↓←→←→BA | Retro Swarm Mode |
| Agent Birthdays | Creation anniversary | Confetti + haiku |
| Swarm Harmony | >95% collaboration | Musical note animation |
| Secret Names | `savant status --secret` | Personality-based codenames |
| Loading Wisdom | Dashboard loading | AI researcher quotes |
| Full Moon | Lunar phase | Temperature +0.1, moon icon |
| Midnight | 12-4 AM | Auto dark theme |
| Achievements | Milestones | 10 hidden badges |
| Swarm's Secret | 100 tasks | Gratitude message |

**Research questions:**

41. What's the most impactful easter egg for user engagement?

42. How do you implement Konami code detection in a React dashboard?

43. Should easter eggs be server-side or client-side?

---

## PART 7: IMPROVEMENT VECTORS

### Quick Wins (High Impact, Low Effort)

1. **Add CORS policy** — Restrict to localhost in gateway
2. **Add request size limits** — Max 1MB per message in gateway
3. **Add background Fjall compaction** — Size-based trigger, 1GB threshold
4. **Fix fetch_message_by_id()** — Add reverse index (message_id → key)
5. **Add WebSocket message coalescing** — Debounce 50ms for high-frequency updates
6. **Split page.tsx into components** — Extract Header, ChatPanel, AgentList, etc.
7. **Add IP rate limiting** — Token bucket per IP in gateway
8. **Add avatar cache persistence** — Save to disk on change
9. **Increase TokenBudget max_tokens** — Allow 1M for hunter-alpha context
10. **Fix heartbeat filesystem sync** — Use async filesystem operations

### Medium Efforts (High Impact, Medium Effort)

11. **EmbeddingService batch processing** — Process 32 texts at once
12. **Entity resolution with NER** — Integrate gline-rs or similar
13. **Daily log rotation automation** — Auto-create new files, archive old
14. **Promotion background worker** — Tokio task scanning for promotion candidates
15. **Dashboard settings page** — Full config UI
16. **Channel-level message queuing** — Persist messages per channel
17. **Skill versioning** — Store versions, allow rollback
8. **Hot-reload using inotify** — Instant detection instead of 2s polling
19. **Add connection pooling** — McpClient reuses connections
20. **Config atomic swap** — Use RwLock with atomic replacement

### Large Efforts (High Impact, High Effort)

21. **DAG compaction (non-destructive)** — Keep original, create compacted copies
22. **Multi-model ensemble with learned quality** — Train quality classifier
23. **Voice interface** — WebRTC + TTS integration
24. **Agent collaboration graph** — Visualize agent interactions
25. **Conversation replay timeline** — Interactive session playback
26. **Per-agent memory partitions** — Optional isolation for personality
27. **Persistent vector index** — Save HNSW to disk, load on startup
28. **Prometheus/Grafana integration** — Metrics endpoints + dashboards
29. **Log rotation system** — Size-based + time-based rotation
30. **Daemon mode** — Background running with PID file

### Architectural Changes (Significant Refactoring)

31. **Decompose SwarmController** — Split into AgentRegistry, ToolRegistry, ProviderManager
32. **Replace rule-based entity extraction with NER** — Full gline-rs integration
33. **Add WASM seccomp profiles** — Restrict syscalls in WASM executor
34. **Implement quantum-safe token minting** — Dilithium2 for tokens
35. **Add proper attestation verification** — TPM + WASM measurement verification
36. **Implement hot-swap compiler** — Live code reload for components
37. **Add global rate limiting** — Per-server, not per-connection
38. **Persistent IPC state** — Save blackboard to disk
39. **Multi-tenant support** — Workspace isolation

---

## PART 8: PRODUCTION BENCHMARKS

Please research benchmarks from similar systems:

### Agent Memory Systems

- **Zep:** Agent memory system with bi-temporal knowledge graph. What are their latency numbers? How do they handle 1M context?
- **Graphiti:** Neo4j-based agent memory. How does graph traversal scale? What are query latencies at 100K nodes?
- **MemGPT:** Virtual context management. What's the overhead of context paging? How do they decide what to keep in context?
- **Hindsight:** Adaptive forgetting. What are the retention periods? How do they balance memory vs forgetting?

### Vector Databases

- **Qdrant:** What are typical HNSW query times at 100K vectors? What's the memory footprint?
- **Pinecone:** Serverless vector search. How do they handle millions of vectors?
- **Weaviate:** Hybrid search. What's the recall/performance trade-off?

### RAG Pipelines

- **LangChain/LlamaIndex:** RAG pipelines. What are typical retrieval latencies? How do they optimize for 1M context?
- **Retrieval accuracy:** What's the best approach for improving recall? Re-ranking? Hybrid search?

### Multi-Agent Systems

- **AutoGen:** Microsoft's multi-agent framework. How do they handle agent communication?
- **CrewAI:** Role-based agents. What's their memory architecture?
- **Swarm (OpenAI):** Handoff-based. How do they share context?

### Embedding Services

- **FastEmbed:** What's the throughput for batch embedding? How does it scale with concurrency?
- **Sentence Transformers:** CPU vs GPU performance. What's the latency for 384-dim vectors?

---

## PART 9: CODE-GROUNDED ANALYSIS

### 9.1 Embedding Bottleneck Analysis

**File:** `savant_core/src/utils/embeddings.rs`

Current implementation uses a single global Mutex:

```rust
pub struct EmbeddingService {
    model: Mutex<TextEmbedding>,
    cache: Mutex<lru::LruCache<String, Vec<f32>>>,
}
```

**Problem:** All concurrent agents serialize on this lock.

**Recommendation:** Create a pool of embedding models:

```rust
pub struct EmbeddingServicePool {
    models: Vec<Mutex<TextEmbedding>>,
    round_robin: AtomicUsize,
}

impl EmbeddingServicePool {
    pub fn new(pool_size: usize) -> Self;
    pub async fn embed(&self, text: &str) -> Vec<f32>;
    pub async fn embed_batch(&self, texts: &[String]) -> Vec<Vec<f32>>;
}
```

### 9.2 Token Budget Analysis

**File:** `savant_agent/src/budget.rs`

Current allocation is fixed percentages:

```rust
pub struct TokenBudget {
    pub system_pct: u8,   // 20%
    pub recent_pct: u8,  // 50%
    pub semantic_pct: u8, // 20%
    pub old_pct: u8,     // 10%
    pub max_tokens: usize, // 256000
}
```

**Problem:** With 1M context window, 256K limit wastes 74% of available context.

**Recommendation:** Allow dynamic allocation based on task:

```rust
pub enum TaskType {
    CodeGeneration,    // High recent, low semantic
    Analysis,          // Balanced
    Conversation,     // High recent
    Research,          // High semantic
}

impl TokenBudget {
    pub fn for_task(task: TaskType, max_tokens: usize) -> Self;
}
```

### 9.3 Fjall Compaction Analysis

**File:** `savant_memory/src/lsm_engine.rs`

Current implementation has no background compaction:

```rust
impl LsmStorageEngine {
    pub fn new(...) -> Result<Arc<Self>, MemoryError> {
        // Opens database, creates keyspaces
        // NO background compaction task
    }
}
```

**Recommendation:** Add background compaction:

```rust
impl LsmStorageEngine {
    pub fn start_compaction_worker(&self, interval: Duration, threshold_bytes: u64);
}
```

### 9.4 Auto-Recall Configuration

**File:** `savant_memory/src/models.rs`

Current defaults:

```rust
pub struct AutoRecallConfig {
    pub enabled: bool,           // true
    pub top_k: usize,            // 5
    pub similarity_threshold: f32, // 0.3
    pub importance_threshold: u8,  // 7
}
```

**Problem:** 5 memories may not be enough for complex tasks. 0.3 threshold may be too loose.

**Recommendation:** Make configurable per-agent:

```rust
impl AgentConfig {
    pub auto_recall: Option<AutoRecallConfig>,
}
```

### 9.5 Vector Persistence

**File:** `savant_memory/src/vector_engine.rs`

Current max vectors: 10M

```rust
const MAX_PERSIST_VECTORS: usize = 10_000_000;
```

**Problem:** 10M vectors × 384 × 4 bytes = 15GB. Plus HNSW overhead.

**Recommendation:** Add tiered storage:

```rust
pub enum VectorStorageTier {
    Hot,      // In-memory HNSW
    Warm,     // mmap'd vectors
    Cold,     // Disk-backed, lazy load
}
```

### 9.6 Security Scanner Patterns

**File:** `savant_skills/src/security.rs`

20+ regex patterns for threat detection. Examples:

```rust
// Credential theft
r"(?i)(api[_-]?key|secret[_-]?key|password|token)['\"]?\s*[:=]\s*['\"]?[\w-]{20,}";

// Reverse shell
r"(?i)(bash\s+-i|nc\s+-e|/dev/tcp|php\s+-r\s+fsockopen)";

// Cryptomining
r"(?i)(stratum\+tcp|coinhive|cryptonight|monero|xmrig)";
```

**Problem:** Regex can be bypassed with obfuscation. No semantic analysis.

**Recommendation:** Add multi-layer scanning:

1. Static regex (fast, catch obvious)
2. AST parsing (deeper analysis)
3. Sandboxed execution (behavioral)

### 9.7 Circuit Breaker Configuration

**File:** `savant_echo/src/circuit_breaker.rs`

Current defaults:

```rust
const DEFAULT_RESET_DURATION_SECS: u64 = 300;  // 5 minutes
const DEFAULT_SUCCESS_THRESHOLD: u64 = 5;       // 5 consecutive successes
```

**Problem:** Static thresholds don't adapt to component behavior.

**Recommendation:** Add adaptive thresholds:

```rust
pub struct AdaptiveThresholds {
    pub min_sample_size: u64,    // Increase if stable
    pub error_threshold: f64,     // Tighten if stable
    pub reset_duration: u64,     // Shorten if frequently resetting
}
```

### 9.8 Provider Retry Logic

**File:** `savant_agent/src/providers/mod.rs`

Current retry behavior:

```rust
// Exponential backoff: 500ms * attempt
tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempts as u64))
```

**Problem:** Fixed backoff doesn't account for rate limits (429) with Retry-After header.

**Recommendation:** Honor Retry-After:

```rust
if let Some(retry_after) = error.retry_after() {
    tokio::time::sleep(Duration::from_secs(retry_after)).await;
}
```

---

## PART 10: PRIORITY RANKINGS

### Top 10 Improvements by Impact

| Rank | Improvement | Impact | Effort | Files to Change |
|------|-------------|--------|--------|-----------------|
| 1 | Increase TokenBudget to 1M | High | Low | savant_agent/budget.rs |
| 2 | Add CORS policy | High | Low | savant_gateway/server.rs |
| 3 | Add request size limits | High | Low | savant_gateway/lanes.rs |
| 4 | Fix fetch_message_by_id | High | Medium | savant_memory/lsm_engine.rs |
| 5 | Background Fjall compaction | High | Medium | savant_memory/lsm_engine.rs |
| 6 | EmbeddingService batch | High | Medium | savant_core/utils/embeddings.rs |
| 7 | Split dashboard page.tsx | Medium | Medium | dashboard/app/page.tsx |
| 8 | Skill hot-reload inotify | Medium | Medium | savant_skills/hot_reload.rs |
| 9 | IP rate limiting | Medium | Medium | savant_gateway/server.rs |
| 10 | Persistent vector index | Medium | High | savant_memory/vector_engine.rs |

### Top 10 Architectural Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Hive mind contamination | High | Add optional partitions |
| DAG compaction data loss | Critical | Make non-destructive |
| OOM from large messages | High | Add size limits |
| WebSocket DoS | Medium | Add rate limiting |
| Token key exposure | High | Implement rotation |
| Quantum break (Ed25519) | Medium | Add Dilithium2 |
| Attestation bypass | High | Implement verification |
| Memory growth unbounded | Medium | Add compaction |
| Skill regex bypass | Medium | Add AST analysis |
| IPC state loss | Low | Add persistence |

### Scalability Roadmap

**10x Scale (500 agents, 1M vectors):**
- EmbeddingService pool (4 instances)
- Vector engine tiered storage
- SessionLane capacity increase
- Fjall compaction enabled

**100x Scale (5000 agents, 10M vectors):**
- Per-agent memory partitions
- Global rate limiting
- Multi-node iceoryx2
- Distributed vector index

**1000x Scale (50000 agents, 100M vectors):**
- Sharded memory
- Hierarchical swarm
- External vector DB (Qdrant)
- Graph database (Graphiti)

---

## PART 11: COMPETITIVE ANALYSIS

### vs. Zep

| Aspect | Savant | Zep |
|--------|--------|-----|
| Storage | Fjall (local) | Cloud |
| Vector | Custom HNSW | Pinecone |
| Context | 1M (hunter-alpha) | 32K |
| Bi-temporal | Yes | Yes |
| Local-first | Yes | No |

**Advantage:** Savant has 30x context, local-first
**Gap:** Zep has better query optimization

### vs. Graphiti

| Aspect | Savant | Graphiti |
|--------|--------|----------|
| Storage | Fjall | Neo4j |
| Graph | DAG nodes | Full RDF |
| Context | 1M | 128K |
| Local-first | Yes | No |

**Advantage:** Savant has 8x context
**Gap:** Graphiti has mature graph queries

### vs. MemGPT

| Aspect | Savant | MemGPT |
|--------|--------|--------|
| Storage | Fjall + vectors | Tiered |
| Paging | None | LLM-managed |
| Context | 1M | 64K |
| Local-first | Yes | No |

**Advantage:** Savant has 15x context, no paging overhead
**Gap:** MemGPT has better context management

### vs. LangChain/LlamaIndex

| Aspect | Savant | LangChain |
|--------|--------|-----------|
| Agents | Yes (swarm) | No |
| Memory | Built-in | External |
| Local-first | Yes | No |
| Context | 1M | Varies |

**Advantage:** Savant is a complete agent platform
**Gap:** LangChain has more integrations

---

## PART 12: RECOMMENDED IMPLEMENTATION ORDER

### Phase 1: Foundation (Week 1-2)

1. Increase TokenBudget to 1M
2. Add CORS policy
3. Add request size limits
4. Add IP rate limiting
5. Increase SessionLane capacity

### Phase 2: Memory (Week 3-4)

6. Fix fetch_message_by_id with index
7. Add background Fjall compaction
8. Persistent vector index
9. Make AutoRecall configurable

### Phase 3: Performance (Week 5-6)

10. EmbeddingService batch + pool
11. Skill hot-reload inotify
12. Split dashboard components

### Phase 4: Safety (Week 7-8)

13. Non-destructive DAG compaction
14. Optional agent memory partitions
15. Skill AST scanning

### Phase 5: Security (Week 9-10)

16. Token rotation
17. Dilithium2 integration
18. Proper attestation

### Phase 6: Scale (Week 11-12)

19. Tiered vector storage
20. SwarmController decomposition
21. Global rate limiting

---

## EXPECTED OUTPUT

Please provide for EACH system:

1. **Current state assessment** — what works, what's broken
2. **Known failure modes** — what goes wrong in production
3. **Optimization opportunities** — specific improvements with trade-offs
4. **Benchmark comparison** — how does this compare to production systems
5. **Priority rating** — 1-10, where 1 = most impactful

Please also provide:

6. **Top 10 improvements** ranked by impact
7. **Architectural risks** — what could cause catastrophic failure
8. **Scalability roadmap** — what happens at 10x, 100x, 1000x scale
9. **Competitive analysis** — how does Savant compare to similar systems
10. **Missing features** — what's the biggest gap vs production systems
11. **Code-level recommendations** — specific functions to change with example code

---

## APPENDIX: FILE REFERENCE

### Core Crates

| File | Lines | Purpose |
|------|-------|---------|
| savant_core/types/mod.rs | 1049 | All shared types |
| savant_core/config.rs | 359 | Configuration |
| savant_core/crypto.rs | 224 | Cryptography |
| savant_core/bus.rs | 195 | Event bus |
| savant_core/db.rs | 263 | Storage |
| savant_core/traits/mod.rs | 105 | Traits |

### Agent Crates

| File | Lines | Purpose |
|------|-------|---------|
| savant_agent/swarm.rs | 596 | Swarm controller |
| savant_agent/providers/mod.rs | 1056 | LLM providers |
| savant_agent/react/mod.rs | 246 | Agent loop |
| savant_agent/context.rs | 159 | Context assembly |
| savant_agent/budget.rs | ~100 | Token budget |

### Memory Crates

| File | Lines | Purpose |
|------|-------|---------|
| savant_memory/vector_engine.rs | 826 | Vector search |
| savant_memory/lsm_engine.rs | 677 | Fjall storage |
| savant_memory/models.rs | 708 | Data models |
| savant_memory/engine.rs | 532 | Memory facade |
| savant_memory/security.rs | 1777 | Security scanner |

### Gateway Crates

| File | Lines | Purpose |
|------|-------|---------|
| savant_gateway/server.rs | 405 | WebSocket server |
| savant_gateway/lanes.rs | 118 | Session lanes |

### Echo Crates

| File | Lines | Purpose |
|------|-------|---------|
| savant_echo/circuit_breaker.rs | 735 | Circuit breaker |

---

*Copy this entire document into Gemini 3.1 Deep Research. The more specific and code-grounded the research, the better we can optimize Savant.*
