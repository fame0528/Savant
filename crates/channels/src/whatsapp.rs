use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::EventFrame;
use std::pin::Pin;
use futures::future::Future;

/// Config for WhatsApp Adapter
pub struct WhatsAppAdapter;

impl ChannelAdapter for WhatsAppAdapter {
    fn name(&self) -> &str {
        "whatsapp"
    }

    fn send_event(&self, event: EventFrame) -> Pin<Box<dyn Future<Output = Result<(), SavantError>> + Send>> {
        Box::pin(async move {
            tracing::warn!("WhatsApp integration pending. Outbound event dropped: {:?}", event.event_type);
            Ok(())
        })
    }

    fn handle_event(&self, event: EventFrame) -> Pin<Box<dyn Future<Output = Result<(), SavantError>> + Send>> {
        Box::pin(async move {
            tracing::info!("WhatsApp incoming event: {:?}", event.event_type);
            Ok(())
        })
    }
}
