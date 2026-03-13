use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::EventFrame;
use async_trait::async_trait;

/// Config for WhatsApp Adapter
pub struct WhatsAppAdapter;

#[async_trait]
impl ChannelAdapter for WhatsAppAdapter {
    fn name(&self) -> &str {
        "whatsapp"
    }

    async fn send_event(&self, event: EventFrame) -> Result<(), SavantError> {
        tracing::warn!("WhatsApp integration pending. Outbound event dropped: {:?}", event.event_type);
        Ok(())
    }

    async fn handle_event(&self, event: EventFrame) -> Result<(), SavantError> {
        tracing::info!("WhatsApp incoming event: {:?}", event.event_type);
        Ok(())
    }
}
