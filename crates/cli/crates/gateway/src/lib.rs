//! Savant CLI Gateway Client
//!
//! WebSocket client that connects to the Savant Gateway for:
//! - Agent chat (LLM inference)
//! - Evolution management (mutations, approvals)
//! - Skill operations (install, enable, disable)
//! - Config management
//! - Swarm telemetry
//!
//! Uses JSON-based ControlFrames reusing `savant_core::types`.

pub mod auth;
pub mod client;
pub mod events;
pub mod handlers;

pub use auth::Ed25519Auth;
pub use client::{ConnectionState, GatewayClient, GatewayConfig};
pub use events::EventRouter;
pub use handlers::ControlFrameHandler;
