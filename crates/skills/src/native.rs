use savant_core::traits::SkillExecutor;
use savant_core::error::SavantError;
use std::pin::Pin;
use futures::future::Future;

/// A trusted native skill implemented statically in rust.
pub struct FileSystemSkill;

impl FileSystemSkill {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileSystemSkill {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillExecutor for FileSystemSkill {
    fn execute(&self, _payload: &str) -> Pin<Box<dyn Future<Output = Result<String, SavantError>> + Send>> {
        Box::pin(async {
            todo!("Execute trusted filesystem tasks.")
        })
    }
}
