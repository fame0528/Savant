#![forbid(unsafe_code)]

//! Savant Gateway Crate
//! WebSocket control plane (axum + tokio-tungstenite).

pub mod auth;
pub mod handlers;
pub mod lanes;
pub mod server;
