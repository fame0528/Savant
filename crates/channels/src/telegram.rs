use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::EventFrame;
use async_trait::async_trait;

pub struct TelegramAdapter;

#[async_trait]
impl ChannelAdapter for TelegramAdapter {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn send_event(&self, event: EventFrame) -> Result<(), SavantError> {
        tracing::info!("Telegram sending event: {:?}", event.event_type);
        Ok(())
    }

    async fn handle_event(&self, event: EventFrame) -> Result<(), SavantError> {
        tracing::info!("Telegram incoming event: {:?}", event.event_type);
        Ok(())
    }
}
