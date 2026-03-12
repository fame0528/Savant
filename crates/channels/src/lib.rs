#![forbid(unsafe_code)]

//! Savant Channels Crate
//! Multi-channel adapters for WhatsApp, Telegram, and CLI.

pub mod cli;
// pub mod matrix;  // Temporarily disabled due to API compatibility issues
pub mod pool;
pub mod telegram;
pub mod whatsapp;
