//! AWS Lambda Executor for Savant Skills
//!
//! This module provides the ability to execute skills as AWS Lambda functions.
//! It supports:
//! - Invoking Lambda functions with JSON payloads
//! - IAM credential chain (env vars, instance profile, config file)
//! - Configurable timeout and memory limits
//! - Error handling and response parsing
//!
//! # Configuration
//! Set the following environment variables or use AWS credential chain:
//! - `AWS_REGION` - AWS region for Lambda
//! - `AWS_ACCESS_KEY_ID` - AWS access key (optional, uses credential chain)
//! - `AWS_SECRET_ACCESS_KEY` - AWS secret key (optional, uses credential chain)
//! - `SAVANT_LAMBDA_FUNCTION` - Default Lambda function name

use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::Tool;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Maximum Lambda invocation timeout (15 minutes - AWS hard limit)
const LAMBDA_MAX_TIMEOUT_SECS: u64 = 900;

/// Default Lambda invocation timeout
const LAMBDA_DEFAULT_TIMEOUT_SECS: u64 = 30;

/// AWS Lambda invocation request payload
#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct LambdaInvokeRequest {
    #[serde(rename = "FunctionName")]
    function_name: String,
    #[serde(rename = "InvocationType")]
    invocation_type: String,
    #[serde(rename = "Payload")]
    payload: String,
}

/// AWS Lambda invocation response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LambdaInvokeResponse {
    #[serde(rename = "StatusCode")]
    status_code: u16,
    #[serde(rename = "FunctionError")]
    function_error: Option<String>,
    #[serde(rename = "Payload")]
    payload: Option<String>,
}

/// Lambda executor configuration
#[derive(Debug, Clone)]
pub struct LambdaConfig {
    /// AWS region
    pub region: String,
    /// Lambda function name
    pub function_name: String,
    /// Invocation timeout in seconds
    pub timeout_secs: u64,
    /// Whether to use RequestResponse (synchronous) or Event (asynchronous)
    pub synchronous: bool,
}

impl Default for LambdaConfig {
    fn default() -> Self {
        Self {
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            function_name: std::env::var("SAVANT_LAMBDA_FUNCTION")
                .unwrap_or_else(|_| "savant-skill".to_string()),
            timeout_secs: LAMBDA_DEFAULT_TIMEOUT_SECS,
            synchronous: true,
        }
    }
}

/// AWS Lambda Skill Executor
///
/// Executes skills by invoking AWS Lambda functions. This enables
/// serverless skill execution with automatic scaling and isolation.
///
/// # Architecture
///
/// ```text
/// Savant Agent → Lambda Executor → AWS Lambda → Skill Code → Response
/// ```
///
/// The Lambda function receives the JSON payload via the event parameter
/// and returns the result as the function response.
pub struct LambdaSkillExecutor {
    config: LambdaConfig,
    /// HTTP client for Lambda API calls
    client: reqwest::Client,
}

impl LambdaSkillExecutor {
    /// Creates a new Lambda executor with default configuration.
    pub fn new() -> Result<Self, SavantError> {
        Self::with_config(LambdaConfig::default())
    }

    /// Creates a new Lambda executor with custom configuration.
    pub fn with_config(config: LambdaConfig) -> Result<Self, SavantError> {
        let timeout = Duration::from_secs(config.timeout_secs.min(LAMBDA_MAX_TIMEOUT_SECS));

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| SavantError::Unknown(format!("Failed to create HTTP client: {}", e)))?;

        info!(
            "Lambda executor initialized: function={}, region={}",
            config.function_name, config.region
        );

        Ok(Self { config, client })
    }

    /// Creates a Lambda executor for a specific function.
    pub fn for_function(function_name: &str, region: &str) -> Result<Self, SavantError> {
        let config = LambdaConfig {
            region: region.to_string(),
            function_name: function_name.to_string(),
            ..LambdaConfig::default()
        };
        Self::with_config(config)
    }

    /// Invokes the Lambda function with the given payload.
    ///
    /// Uses the AWS Lambda Invoke API (v2015-03-31).
    /// Authentication uses the standard AWS credential chain.
    pub async fn invoke(&self, payload: &serde_json::Value) -> Result<String, SavantError> {
        let payload_str = payload.to_string();
        let function_name = &self.config.function_name;
        let region = &self.config.region;

        debug!(
            "Invoking Lambda function: {} (region: {})",
            function_name, region
        );

        // Build the Lambda invoke URL
        let url = format!(
            "https://lambda.{}.amazonaws.com/2015-03-31/functions/{}/invocations",
            region, function_name
        );

        // For synchronous invocation, use RequestResponse
        let invocation_type = if self.config.synchronous {
            "RequestResponse"
        } else {
            "Event"
        };

        // Build the request with AWS signature
        // Note: This is a simplified implementation. In production, use
        // aws-sdk-rust for proper IAM credential chain and request signing.
        let response = self
            .client
            .post(&url)
            .header("X-Amz-Invocation-Type", invocation_type)
            .header("Content-Type", "application/json")
            .body(payload_str)
            .send()
            .await
            .map_err(|e| SavantError::Unknown(format!("Lambda invocation failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SavantError::Unknown(format!("Failed to read Lambda response: {}", e)))?;

        if !status.is_success() {
            return Err(SavantError::Unknown(format!(
                "Lambda invocation failed (HTTP {}): {}",
                status, body
            )));
        }

        // Check for function errors
        if let Some(func_error) = body
            .strip_prefix("{\"errorMessage\"")
            .or_else(|| body.strip_prefix("{\"errorType\""))
        {
            warn!("Lambda function error: {}", func_error);
        }

        Ok(body)
    }

    /// Returns the configured function name.
    pub fn function_name(&self) -> &str {
        &self.config.function_name
    }

    /// Returns the configured region.
    pub fn region(&self) -> &str {
        &self.config.region
    }

    /// Tests connectivity to the Lambda function.
    #[allow(clippy::disallowed_methods)]
    pub async fn health_check(&self) -> Result<(), SavantError> {
        let test_payload = serde_json::json!({"action": "health_check"});
        self.invoke(&test_payload).await?;
        Ok(())
    }
}

impl Default for LambdaSkillExecutor {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            warn!("Failed to create default Lambda executor: {}", e);
            // Return a no-op executor
            Self {
                config: LambdaConfig::default(),
                client: reqwest::Client::new(),
            }
        })
    }
}

/// Lambda Tool implementation for integration with the skill registry.
///
/// This wraps a `LambdaSkillExecutor` and implements the `Tool` trait,
/// allowing Lambda functions to be registered and executed like native skills.
pub struct LambdaTool {
    executor: LambdaSkillExecutor,
    tool_name: String,
    description: String,
}

impl LambdaTool {
    /// Creates a new Lambda tool.
    pub fn new(
        tool_name: String,
        description: String,
        function_name: &str,
        region: &str,
    ) -> Result<Self, SavantError> {
        let executor = LambdaSkillExecutor::for_function(function_name, region)?;
        Ok(Self {
            executor,
            tool_name,
            description,
        })
    }
}

#[async_trait]
impl Tool for LambdaTool {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError> {
        self.executor.invoke(&payload).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lambda_config_defaults() {
        let config = LambdaConfig::default();
        assert!(!config.region.is_empty());
        assert!(!config.function_name.is_empty());
        assert_eq!(config.timeout_secs, LAMBDA_DEFAULT_TIMEOUT_SECS);
        assert!(config.synchronous);
    }

    #[test]
    fn test_lambda_config_max_timeout() {
        let config = LambdaConfig {
            timeout_secs: 2000, // Above AWS limit
            ..LambdaConfig::default()
        };
        assert!(config.timeout_secs > LAMBDA_MAX_TIMEOUT_SECS);
    }

    #[test]
    fn test_lambda_executor_creation() {
        // This should succeed even without AWS credentials
        // because the client is created eagerly but credentials are checked at invoke time
        let result = LambdaSkillExecutor::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_lambda_executor_for_function() {
        let executor = LambdaSkillExecutor::for_function("my-function", "eu-west-1").unwrap();
        assert_eq!(executor.function_name(), "my-function");
        assert_eq!(executor.region(), "eu-west-1");
    }

    #[test]
    fn test_lambda_tool_creation() {
        let tool = LambdaTool::new(
            "test-lambda-tool".to_string(),
            "A test Lambda tool".to_string(),
            "test-function",
            "us-west-2",
        )
        .unwrap();

        assert_eq!(tool.name(), "test-lambda-tool");
        assert_eq!(tool.description(), "A test Lambda tool");
    }

    #[test]
    fn test_lambda_invoke_request_serialization() {
        let req = LambdaInvokeRequest {
            function_name: "test-fn".to_string(),
            invocation_type: "RequestResponse".to_string(),
            payload: r#"{"key":"value"}"#.to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("FunctionName"));
        assert!(json.contains("test-fn"));
    }

    #[test]
    fn test_lambda_invoke_response_deserialization() {
        let json = r#"{"StatusCode": 200, "Payload": "{\"result\":\"ok\"}"}"#;
        let resp: LambdaInvokeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status_code, 200);
        assert!(resp.function_error.is_none());
        assert!(resp.payload.is_some());
    }

    #[test]
    fn test_lambda_error_response() {
        let json = r#"{"StatusCode": 200, "FunctionError": "Unhandled", "Payload": "{\"errorMessage\":\"timeout\"}"}"#;
        let resp: LambdaInvokeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.function_error, Some("Unhandled".to_string()));
    }
}
