//! SkillManagerTool — exposes SkillManager operations as an agent tool.
//!
//! Allows agents to list, discover, approve, reject, and manage skills
//! through the standard Tool trait interface.
// SAFETY: All `clippy::disallowed_methods` violations in this file originate from
// the `serde_json::json!()` macro, which internally uses `.unwrap()` on
// compile-time-validated JSON literals. A malformed JSON literal would be a
// compile error, making the panic path statically unreachable.
#![allow(clippy::disallowed_methods)]

use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::Tool;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tool for managing the skill lifecycle — discovery, approval, rejection, and listing.
pub struct SkillManagerTool {
    skill_manager: Arc<Mutex<savant_skills::parser::SkillManager>>,
}

impl SkillManagerTool {
    pub fn new(skill_manager: Arc<Mutex<savant_skills::parser::SkillManager>>) -> Self {
        Self { skill_manager }
    }
}

#[async_trait]
impl Tool for SkillManagerTool {
    fn name(&self) -> &str {
        "skill_manager"
    }

    fn description(&self) -> &str {
        "Manage agent skills: list discovered skills, check pending approvals, approve or reject skills, and trigger discovery. Use this to inspect available skills and manage their lifecycle."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "pending", "approve", "reject", "discover"],
                    "description": "Action to perform on the skill manager"
                },
                "skill_name": {
                    "type": "string",
                    "description": "Name of the skill (required for approve/reject actions)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, payload: Value) -> Result<String, SavantError> {
        let action = payload["action"]
            .as_str()
            .ok_or_else(|| SavantError::Unknown("Missing 'action' field".to_string()))?;

        let skill_name = payload["skill_name"].as_str();

        match action {
            "list" => {
                let manager = self.skill_manager.lock().await;
                let skills = manager.list_skills();
                if skills.is_empty() {
                    return Ok(
                        "No skills discovered. Use action 'discover' to scan for skills."
                            .to_string(),
                    );
                }
                let mut result = String::from("Discovered skills:\n");
                for (name, meta) in &skills {
                    let status = if meta.enabled { "enabled" } else { "disabled" };
                    let trust = format!("{:?}", meta.trust_tier);
                    result.push_str(&format!(
                        "- {} ({}, {}): {}\n",
                        name, status, trust, meta.source
                    ));
                }
                Ok(result)
            }
            "pending" => {
                let manager = self.skill_manager.lock().await;
                let pending = manager.get_pending_approvals();
                if pending.is_empty() {
                    return Ok("No skills pending approval.".to_string());
                }
                let mut result = String::from("Skills pending approval:\n");
                for (name, gate) in &pending {
                    result.push_str(&format!(
                        "- {} (risk: {:?}, progress: {:.0}%)\n",
                        name,
                        gate.scan_result().risk_level,
                        gate.approval_progress() * 100.0
                    ));
                }
                Ok(result)
            }
            "approve" => {
                let name = skill_name.ok_or_else(|| {
                    SavantError::Unknown("Missing 'skill_name' for approve action".to_string())
                })?;
                let mut manager = self.skill_manager.lock().await;
                manager.approve_pending_skill(name).await?;
                Ok(format!("Skill '{}' approved.", name))
            }
            "reject" => {
                let name = skill_name.ok_or_else(|| {
                    SavantError::Unknown("Missing 'skill_name' for reject action".to_string())
                })?;
                let mut manager = self.skill_manager.lock().await;
                manager.reject_pending_skill(name)?;
                Ok(format!("Skill '{}' rejected.", name))
            }
            "discover" => {
                let mut manager = self.skill_manager.lock().await;
                let result = manager.discover_all_skills(None).await?;
                Ok(format!(
                    "Discovery complete: {} swarm skills, {} agent skills",
                    result.swarm_skills, result.agent_skills
                ))
            }
            _ => Err(SavantError::Unknown(format!(
                "Unknown action '{}'. Valid actions: list, pending, approve, reject, discover",
                action
            ))),
        }
    }

    fn capabilities(&self) -> savant_core::types::CapabilityGrants {
        savant_core::types::CapabilityGrants {
            fs_read: [std::path::PathBuf::from("skills")].into_iter().collect(),
            fs_write: [std::path::PathBuf::from("skills")].into_iter().collect(),
            ..Default::default()
        }
    }
}
