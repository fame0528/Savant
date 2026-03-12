use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::EventFrame;
use std::pin::Pin;
use futures::future::Future;

pub struct TelegramAdapter;

impl ChannelAdapter for TelegramAdapter {
    fn name(&self) -> &str {
        "telegram"
    }

    fn send_event(&self, _event: EventFrame) -> Pin<Box<dyn Future<Output = Result<(), SavantError>> + Send>> {
        Box::pin(async move {
            tracing::info!("Telegram sending event: {:?}", _event.event_type);
            Ok(())
        })
    }

    fn handle_event(&self, event: EventFrame) -> Pin<Box<dyn Future<Output = Result<(), SavantError>> + Send>> {
        Box::pin(async move {
            tracing::info!("Telegram incoming event: {:?}", event.event_type);
            Ok(())
        })
    }
}
