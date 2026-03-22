use crate::react::AgentLoop;
use savant_core::error::SavantError;
use savant_core::traits::MemoryBackend;
use tracing::{info, warn};

impl<M: MemoryBackend> AgentLoop<M> {
    /// Verifies if the current agent has the required security token to execute a specific tool.
    pub(crate) fn verify_tool_access(&self, tool_name: &str) -> Result<(), SavantError> {
        info!(
            "Security Enclave: Verifying access for tool [{}] for agent [{}]",
            tool_name, self.agent_id
        );

        // 🛡️ AAA Logic: If security is enabled (token present), enforce it.
        if let Some(token) = &self.security_token {
            if !token.assignee_matches(self.agent_id_hash) {
                warn!(
                    "CCT VIOLATION: Token assignee mismatch for agent [{}]",
                    self.agent_id
                );
                return Err(SavantError::AuthError(
                    "Security token binding failed: ID mismatch".into(),
                ));
            }

            let resource = format!("savant://tools/{}", tool_name);
            if !token.verify_capability(&resource, "execute") {
                warn!(
                    "CCT VIOLATION: Permitted action 'execute' denied for resource [{}]",
                    resource
                );
                return Err(SavantError::AuthError(format!(
                    "Capability denied for tool: {}",
                    tool_name
                )));
            }
            info!("Security Enclave: Access GRANTED for tool [{}]", tool_name);
        } else {
            warn!(
                "SECURITY WARNING: Agent [{}] executing tool [{}] without a CCT.",
                self.agent_id, tool_name
            );
        }
        Ok(())
    }

    pub(crate) async fn execute_tool(&self, name: &str, args: &str) -> Result<String, SavantError> {
        // CRITICAL: Full cryptographic verification via SecurityAuthority when available
        if let (Some(token), Some(authority)) = (&self.security_token, &self.security_authority) {
            let resource = format!("savant://tools/{}", name);
            authority
                .verify_token_and_action(token, self.agent_id_hash, &resource, "execute")
                .map_err(|e| {
                    SavantError::AuthError(format!("CCT Crypto Verification Failed: {}", e))
                })?;
            info!(
                "Security Enclave: Full crypto verification GRANTED for tool [{}]",
                name
            );
        } else {
            // Fallback: lightweight assignee + capability check
            self.verify_tool_access(name)?;
        }

        for tool in &self.tools {
            if tool.name().to_lowercase() == name.to_lowercase() {
                let mut payload = serde_json::from_str(args)
                    .unwrap_or_else(|_| serde_json::json!({ "payload": args }));

                // Coerce arguments against tool's JSON Schema
                let schema = tool.parameters_schema();
                if schema.get("type").is_some() {
                    payload = crate::tools::coercion::prepare_tool_params(&payload, &schema);
                }

                // Execute with timeout
                let timeout_secs = tool.timeout_secs();
                let max_output = tool.max_output_chars();
                let tool_clone = tool.clone();
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(timeout_secs),
                    self.hyper_causal.execute_speculative(tool_clone, payload),
                )
                .await;

                return match result {
                    Ok(inner_result) => {
                        inner_result.map(|output| truncate_output(&output, max_output))
                    }
                    Err(_) => Err(SavantError::Unknown(format!(
                        "Tool '{}' timed out after {} seconds",
                        name, timeout_secs
                    ))),
                };
            }
        }

        if let (Some(registry), Some(host)) = (&self.echo_registry, &self.echo_host) {
            if let Some(capability) = registry.get_tool(name) {
                match host.execute_tool(&capability.module, args).await {
                    Ok(res) => {
                        if let Some(metrics) = &self.echo_metrics {
                            metrics.record_outcome(true);
                        }
                        return Ok(res);
                    }
                    Err(e) => {
                        if let Some(metrics) = &self.echo_metrics {
                            metrics.record_outcome(false);
                        }
                        return Err(SavantError::Unknown(e.to_string()));
                    }
                }
            }
        }
        Err(SavantError::Unknown(format!("Tool not found: {}", name)))
    }

    /// OMEGA: Heuristic Resolution Matrix.
    /// Handles tool failures by attempting specific resolution paths.
    pub(crate) async fn handle_heuristic_resolution(
        &mut self,
        tool_name: &str,
        error: SavantError,
    ) -> Result<String, SavantError> {
        info!(
            "[{}] HEURISTIC: Triggering resolution path for tool [{}] failure: {:?}",
            self.agent_id, tool_name, error
        );
        self.heuristic.failures += 1;

        match self.heuristic.failures {
            1 => {
                // Path 1: Contextual Expansion
                info!(
                    "[{}] HEURISTIC: Path 1 - Contextual Expansion triggered.",
                    self.agent_id
                );
                Ok("Recovery hint: Try to re-read the documentation for the tool and verify arguments.".to_string())
            }
            2 => {
                // Path 2: Technical Refinement (Rollback)
                if let Some(checkpoint) = self.heuristic.last_stable_checkpoint.take() {
                    info!("[{}] HEURISTIC: Path 2 - Triggering state rollback to last stable checkpoint ({} messages).", self.agent_id, checkpoint.len());
                    // In a real loop, we'd reset the 'history' here.
                    // For now, we signal the loop to retry with a fresh prompt.
                    Ok("Recovery hint: System state inconsistent. Rolling back to last stable checkpoint. Please simplify the request.".to_string())
                } else {
                    Ok("Recovery hint: Attempting alternate tool strategy.".to_string())
                }
            }
            _ => {
                // Path 3: Architectural Pivot
                warn!(
                    "[{}] HEURISTIC: Maximum retries reached. Failing session.",
                    self.agent_id
                );
                Err(SavantError::HeuristicFailure(format!(
                    "Recursive failure loop detected for tool: {}",
                    tool_name
                )))
            }
        }
    }
}

/// Truncate tool output with head+tail preservation.
fn truncate_output(output: &str, max_chars: usize) -> String {
    if output.len() <= max_chars {
        return output.to_string();
    }
    let head_size = (max_chars * 60) / 100;
    let tail_size = (max_chars * 40) / 100;
    format!(
        "{}\n\n[... {} chars truncated ...]\n\n{}",
        &output[..head_size],
        output.len() - max_chars,
        &output[output.len() - tail_size..]
    )
}
