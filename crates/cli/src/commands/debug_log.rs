use savant_core::bus::debug_log_sender;

/// Tracing layer that publishes log messages to the global debug log channel
/// for real-time streaming to the dashboard via WebSocket.
pub struct DebugLogLayer;

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for DebugLogLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = LogVisitor::default();
        event.record(&mut visitor);
        let meta = event.metadata();
        let msg = format!("[{}] {}", meta.level(), visitor.message);
        // No-receiver error is expected when debug WebSocket isn't connected
        drop(debug_log_sender().send(msg));
    }
}

#[derive(Default)]
pub struct LogVisitor {
    pub message: String,
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
}
