#![allow(clippy::disallowed_methods)]
//! Verified Hybrid Semantic Substrate (VHSS)
//!
//! This crate implements a production-grade memory subsystem that combines:
//! - Fjall 3.0 LSM-tree for transactional, high-concurrency persistence
//! - ruvector-core for SIMD-accelerated semantic search
//! - rkyv for zero-copy serialization
//! - Formal Kani verification for memory safety
//!
//! It completely eliminates OpenClaw's race conditions, ZeroClaw's memory bleed,
//! and provides mathematically proven safety guarantees.

pub mod arbiter;
mod async_backend;
pub mod daily_log;
pub mod distillation;
mod engine;
pub mod entities;
mod error;
mod lsm_engine;
pub mod models;
pub mod notifications;
pub mod promotion;
pub mod safety;
mod vector_engine;

pub use async_backend::AsyncMemoryBackend;
pub use daily_log::{DailyLog, LogEntry, LogPriority};
pub use engine::MemoryEngine;
pub use entities::{Entity, EntityExtractor, EntityType};
pub use error::MemoryError;
pub use lsm_engine::{LsmStorageEngine, StorageStats};
pub use models::{
    message_key, session_key, session_state_key, turn_state_key, verify_tool_pair_integrity,
    AgentMessage, MemoryEntry, MessageRole, SessionState, ToolCallRef, ToolResultRef, TurnPhase,
    TurnState,
};
pub use notifications::{MemoryNotification, NotificationChannel};
pub use promotion::{PersonalityTraits, PromotionEngine, PromotionMetrics};
pub use savant_core::utils::embeddings::EmbeddingService;
// Safety verification module is conditionally compiled with kani feature
#[cfg(feature = "kani")]
pub use safety::verify_memory_safety;
pub use vector_engine::SemanticVectorEngine;
