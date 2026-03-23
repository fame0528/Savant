use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::EventFrame;

pub struct CliAdapter;

#[async_trait]
impl ChannelAdapter for CliAdapter {
    fn name(&self) -> &str {
        "cli"
    }

    async fn send_event(&self, event: EventFrame) -> Result<(), SavantError> {
        tracing::info!("[CLI] {}", event.payload);
        Ok(())
    }

    async fn handle_event(&self, event: EventFrame) -> Result<(), SavantError> {
        tracing::info!("CLI incoming event: {:?}", event.event_type);
        Ok(())
    }
}
