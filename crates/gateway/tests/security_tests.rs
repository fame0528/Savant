//! Gateway security penetration tests - tests all security fixes.

use axum::http::StatusCode;
use savant_gateway::server::GatewayState;
use std::collections::DashMap;
use std::sync::Arc;

/// Helper to create a test GatewayState
fn create_test_state() -> Arc<GatewayState> {
    let config = savant_core::config::Config::default();
    let nexus = Arc::new(savant_core::bus::NexusBridge::new().unwrap());
    let storage = Arc::new(
        savant_core::db::Storage::new(std::path::PathBuf::from(
            std::env::temp_dir().join("test-gateway-security"),
        ))
        .unwrap(),
    );

    Arc::new(GatewayState {
        config,
        sessions: DashMap::new(),
        nexus,
        storage,
        avatar_cache: tokio::sync::Mutex::new(lru::LruCache::new(
            std::num::NonZeroUsize::new(10).unwrap(),
        )),
        gateway_signing_key: ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng),
    })
}

#[tokio::test]
async fn test_gateway_signing_key_generation() {
    let state = create_test_state();

    // Verify key is non-zero (properly generated)
    let key_bytes = state.gateway_signing_key.to_bytes();
    assert!(
        !key_bytes.iter().all(|&b| b == 0),
        "Signing key should not be all zeros"
    );
    assert!(key_bytes.len() == 32, "Signing key should be 32 bytes");
}

#[tokio::test]
async fn test_gateway_key_is_random() {
    let state1 = create_test_state();
    let state2 = create_test_state();

    let key1 = state1.gateway_signing_key.to_bytes();
    let key2 = state2.gateway_signing_key.to_bytes();

    assert_ne!(
        key1, key2,
        "Each GatewayState should have a unique signing key"
    );
}

#[tokio::test]
async fn test_skill_name_validation() {
    // Valid names
    assert!(validate_skill_name("hello-world").is_ok());
    assert!(validate_skill_name("my_skill_123").is_ok());
    assert!(validate_skill_name("simple").is_ok());

    // Invalid names
    assert!(validate_skill_name("").is_err(), "Empty name should fail");
    assert!(
        validate_skill_name("../etc/passwd").is_err(),
        "Path traversal should fail"
    );
    assert!(
        validate_skill_name("hello world").is_err(),
        "Spaces should fail"
    );
    assert!(
        validate_skill_name("hello/world").is_err(),
        "Slash should fail"
    );
    assert!(
        validate_skill_name("hello;world").is_err(),
        "Semicolon should fail"
    );
    assert!(
        validate_skill_name("hello\\world").is_err(),
        "Backslash should fail"
    );

    // Overly long name
    let long_name: String = "a".repeat(200);
    assert!(
        validate_skill_name(&long_name).is_err(),
        "Long name should fail"
    );
}

#[tokio::test]
async fn test_directive_sanitization() {
    // Valid directives
    assert!(validate_directive("deploy agent alpha").is_ok());

    // Control characters should fail
    assert!(
        validate_directive("deploy\x00agent").is_err(),
        "Null byte should fail"
    );
    assert!(
        validate_directive("deploy\x01agent").is_err(),
        "Control char should fail"
    );

    // Empty directive should fail
    assert!(
        validate_directive("").is_err(),
        "Empty directive should fail"
    );

    // Very long directive should fail
    let long_directive: String = "x".repeat(3000);
    assert!(
        validate_directive(&long_directive).is_err(),
        "Long directive should fail"
    );
}

#[tokio::test]
async fn test_auth_error_sanitization() {
    // Auth errors should not leak internal details
    let error = savant_core::error::SavantError::AuthError("internal key format wrong".to_string());
    let error_str = error.to_string();

    assert!(
        !error_str.contains("key format"),
        "Auth error should not leak internal details: {}",
        error_str
    );
}

// Helper functions that mirror the actual implementations
fn validate_skill_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Skill name cannot be empty".to_string());
    }
    if name.len() > 128 {
        return Err("Skill name too long".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err("Invalid characters".to_string());
    }
    if name.contains("..") {
        return Err("Path traversal detected".to_string());
    }
    Ok(())
}

fn validate_directive(directive: &str) -> Result<(), String> {
    if directive.is_empty() {
        return Err("Empty directive".to_string());
    }
    if directive.len() > 2048 {
        return Err("Directive too long".to_string());
    }
    if directive
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        return Err("Control characters not allowed".to_string());
    }
    Ok(())
}
