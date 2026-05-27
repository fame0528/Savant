//! HTTP Authentication Middleware
//!
//! Provides axum Tower middleware for REST API authentication.
//! Validates `Authorization: Bearer <key>` or `X-API-Key: <key>` headers
//! against the configured `dashboard_api_key`.
//!
//! Health/readiness endpoints (`/health`, `/live`, `/ready`) and WebSocket
//! routes (`/ws`, `/ws/canvas`) are exempt from authentication.

#![allow(clippy::disallowed_methods)] // json!() macro internal .unwrap() is provably infallible

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Paths that do not require authentication.
const PUBLIC_PATHS: &[&str] = &["/health", "/live", "/ready", "/ws", "/ws/canvas"];

/// Constant-time byte comparison to prevent timing attacks.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Extract the API key from request headers.
///
/// Supports two header formats:
/// - `Authorization: Bearer <key>`
/// - `X-API-Key: <key>`
fn extract_api_key(req: &Request<Body>) -> Option<String> {
    // Try Authorization: Bearer <key>
    if let Some(auth_header) = req.headers().get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(key) = auth_str.strip_prefix("Bearer ") {
                return Some(key.to_string());
            }
        }
    }

    // Try X-API-Key: <key>
    if let Some(api_key_header) = req.headers().get("x-api-key") {
        if let Ok(key) = api_key_header.to_str() {
            return Some(key.to_string());
        }
    }

    None
}

/// Authentication middleware for REST API endpoints.
///
/// Validates the API key from request headers against the configured
/// `dashboard_api_key`. Returns 401 Unauthorized if the key is missing
/// or invalid.
pub async fn auth_middleware(
    axum::extract::State(expected_key): axum::extract::State<Option<String>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path();

    // Allow public endpoints without authentication
    if PUBLIC_PATHS.iter().any(|p| path == *p || path.starts_with(&format!("{p}/"))) {
        return next.run(req).await;
    }

    // If no dashboard API key is configured, allow all requests (development mode)
    let expected = match expected_key {
        Some(key) if !key.is_empty() => key,
        _ => return next.run(req).await,
    };

    // Extract API key from request
    let provided_key = extract_api_key(&req);

    match provided_key {
        Some(key) if constant_time_eq(key.as_bytes(), expected.as_bytes()) => {
            next.run(req).await
        }
        _ => {
            tracing::warn!(
                "[auth] Unauthorized request to {} from {}",
                path,
                req.headers()
                    .get("x-forwarded-for")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown")
            );
            (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({
                    "error": "Unauthorized",
                    "message": "Valid API key required. Provide via 'Authorization: Bearer <key>' or 'X-API-Key: <key>' header."
                })),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_eq_equal() {
        assert!(constant_time_eq(b"secret123", b"secret123"));
    }

    #[test]
    fn test_constant_time_eq_different() {
        assert!(!constant_time_eq(b"secret123", b"secret124"));
    }

    #[test]
    fn test_constant_time_eq_different_lengths() {
        assert!(!constant_time_eq(b"short", b"muchlongervalue"));
    }

    #[test]
    fn test_constant_time_eq_empty() {
        assert!(constant_time_eq(b"", b""));
    }
}
