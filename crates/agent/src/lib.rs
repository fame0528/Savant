#![forbid(unsafe_code)]

//! Savant Agent Crate
//! Contains the ReAct loop, LLM providers, and token budgeting.

pub mod budget;
pub mod compact;
pub mod consciousness;
pub mod context;
pub mod context_compressor;
pub mod ensemble;
pub mod free_model_router;
pub mod governor;
pub mod graceful_shutdown;
pub mod learning;
pub mod lsp;
pub mod manager;
pub mod memory;
pub mod nlp;
pub mod orchestration;
pub mod plugins;
pub mod proactive;
pub mod prompts;
pub mod providers;
pub mod pulse;
pub mod rate_limiter;
pub mod react;
pub mod react_speculative;
pub mod semantic_window;
pub mod shell_intel;
pub mod soul_examples;
pub mod swarm;
pub mod tools;
pub mod watcher;
pub mod workspace;
