use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::EventFrame;
use std::pin::Pin;
use futures::future::Future;

pub struct CliAdapter;

impl ChannelAdapter for CliAdapter {
    fn name(&self) -> &str {
        "cli"
    }

    fn send_event(&self, _event: EventFrame) -> Pin<Box<dyn Future<Output = Result<(), SavantError>> + Send>> {
        Box::pin(async move {
            println!("[CLI] {}", _event.payload);
            Ok(())
        })
    }

    fn handle_event(&self, event: EventFrame) -> Pin<Box<dyn Future<Output = Result<(), SavantError>> + Send>> {
        Box::pin(async move {
            tracing::info!("CLI incoming event: {:?}", event.event_type);
            Ok(())
        })
    }
}
