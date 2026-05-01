use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};
use tracing::debug;

/// Control mode for human-agent browser collaboration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ControlMode {
    /// Only human can control the browser.
    Human = 0,
    /// Only agents can control the browser.
    Agent = 1,
    /// Both human and agent can control; last action wins with notification.
    Collaborative = 2,
}

impl ControlMode {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => ControlMode::Human,
            1 => ControlMode::Agent,
            _ => ControlMode::Collaborative,
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            ControlMode::Human => 0,
            ControlMode::Agent => 1,
            ControlMode::Collaborative => 2,
        }
    }
}

impl Default for ControlMode {
    fn default() -> Self {
        ControlMode::Collaborative
    }
}

/// Lock-free control mode manager using atomic operations.
pub struct ControlModeManager {
    mode: AtomicU8,
}

impl ControlModeManager {
    pub fn new(initial: ControlMode) -> Self {
        Self {
            mode: AtomicU8::new(initial.as_u8()),
        }
    }

    pub fn default() -> Self {
        Self::new(ControlMode::default())
    }

    /// Sets the control mode and returns the previous mode.
    pub fn set_mode(&self, mode: ControlMode) -> ControlMode {
        let prev = self.mode.swap(mode.as_u8(), Ordering::SeqCst);
        debug!(
            "[browser::control] Mode changed: {:?} -> {:?}",
            ControlMode::from_u8(prev),
            mode
        );
        ControlMode::from_u8(prev)
    }

    /// Gets the current control mode.
    pub fn get_mode(&self) -> ControlMode {
        ControlMode::from_u8(self.mode.load(Ordering::SeqCst))
    }

    /// Returns true if agents are allowed to act in the current mode.
    pub fn can_agent_act(&self) -> bool {
        matches!(
            self.get_mode(),
            ControlMode::Agent | ControlMode::Collaborative
        )
    }

    /// Returns true if humans are allowed to act in the current mode.
    pub fn can_human_act(&self) -> bool {
        matches!(
            self.get_mode(),
            ControlMode::Human | ControlMode::Collaborative
        )
    }

    /// Resolves a navigation conflict based on the current mode.
    /// Returns whether the requested action should proceed.
    pub fn resolve_conflict(&self, requested_by_agent: bool) -> bool {
        match self.get_mode() {
            ControlMode::Human => !requested_by_agent,
            ControlMode::Agent => requested_by_agent,
            ControlMode::Collaborative => true, // Last-wins in collaborative mode
        }
    }
}

/// Browser annotation for agent highlights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAnnotation {
    pub id: String,
    pub tab_id: String,
    pub selector: String,
    pub text: String,
    pub agent_name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl BrowserAnnotation {
    pub fn new(tab_id: String, selector: String, text: String, agent_name: String) -> Self {
        use uuid::Uuid;
        Self {
            id: Uuid::new_v4().to_string(),
            tab_id,
            selector,
            text,
            agent_name,
            timestamp: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_mode_default() {
        let mgr = ControlModeManager::default();
        assert!(mgr.can_agent_act());
        assert!(mgr.can_human_act());
    }

    #[test]
    fn test_human_mode() {
        let mgr = ControlModeManager::new(ControlMode::Human);
        assert!(!mgr.can_agent_act());
        assert!(mgr.can_human_act());
        assert!(mgr.resolve_conflict(false)); // human navigates
        assert!(!mgr.resolve_conflict(true)); // agent blocked
    }

    #[test]
    fn test_agent_mode() {
        let mgr = ControlModeManager::new(ControlMode::Agent);
        assert!(mgr.can_agent_act());
        assert!(!mgr.can_human_act());
        assert!(!mgr.resolve_conflict(false)); // human blocked
        assert!(mgr.resolve_conflict(true)); // agent navigates
    }

    #[test]
    fn test_collaborative_mode() {
        let mgr = ControlModeManager::new(ControlMode::Collaborative);
        assert!(mgr.can_agent_act());
        assert!(mgr.can_human_act());
        assert!(mgr.resolve_conflict(false)); // human OK
        assert!(mgr.resolve_conflict(true)); // agent OK
    }

    #[test]
    fn test_set_mode() {
        let mgr = ControlModeManager::default();
        let prev = mgr.set_mode(ControlMode::Human);
        assert_eq!(prev, ControlMode::Collaborative);
        assert!(!mgr.can_agent_act());
    }

    #[test]
    fn test_annotation_creation() {
        let ann = BrowserAnnotation::new(
            "tab-1".to_string(),
            "#submit-btn".to_string(),
            "Click this to submit".to_string(),
            "agent-alpha".to_string(),
        );
        assert!(!ann.id.is_empty());
        assert_eq!(ann.tab_id, "tab-1");
        assert_eq!(ann.selector, "#submit-btn");
        assert_eq!(ann.agent_name, "agent-alpha");
    }
}
