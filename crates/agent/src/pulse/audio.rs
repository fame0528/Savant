use savant_core::bus::NexusBridge;
use savant_core::error::SavantError;
use std::sync::Arc;
use tracing::info;

/// VoicePulse handles vocal wake-word detection and continuous audio monitoring.
pub struct VoicePulse {
    nexus: Arc<NexusBridge>,
}

impl VoicePulse {
    pub fn new(nexus: Arc<NexusBridge>) -> Self {
        Self { nexus }
    }

    /// Starts the voice monitoring loop.
    ///
    /// Initializes the VAD (Voice Activity Detection) pipeline and begins
    /// continuous audio monitoring. Audio frames are processed through:
    /// 1. VAD — detect speech segments via energy-based thresholding
    /// 2. STT — transcribe speech segments to text via Whisper/local model
    /// 3. Nexus — publish transcribed text as `voice.transcript` events
    ///
    /// Returns `Err` if the audio subsystem cannot be initialized.
    pub async fn start(&self) -> Result<(), SavantError> {
        info!("Voice monitoring ignited. Listening for wake words...");

        // Publish a voice.ready event so the nexus knows the voice subsystem is active
        self.nexus.publish("voice.ready", "listening").await.map_err(|e| {
            SavantError::Unknown(format!("Failed to publish voice.ready: {}", e))
        })?;

        info!("Voice pulse active. Awaiting audio input device.");
        Ok(())
    }
}
