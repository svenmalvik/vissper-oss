//! Audio types and error definitions

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use tracing::info;

/// Audio chunk ready to be sent over WebSocket
///
/// Contains PCM audio data optimized for real-time transcription.
/// The sample rate is typically 16kHz after resampling.
#[derive(Debug, Clone)]
pub struct AudioChunk {
    /// PCM 16-bit signed samples (mono)
    pub samples: Vec<i16>,
    /// Sample rate in Hz (typically 16000)
    pub sample_rate: u32,
}

/// Handle for controlling audio capture from outside the capture thread
///
/// Provides methods to stop capturing and check the capture status.
/// The capture automatically stops when this handle is dropped.
pub struct AudioCaptureHandle {
    pub(crate) is_capturing: Arc<AtomicBool>,
    pub(crate) thread_handle: Option<JoinHandle<()>>,
}

impl AudioCaptureHandle {
    /// Stop capturing audio
    pub fn stop(&mut self) {
        self.is_capturing.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        info!("Audio capture stopped");
    }

    /// Check if currently capturing
    #[allow(dead_code)]
    pub fn is_capturing(&self) -> bool {
        self.is_capturing.load(Ordering::SeqCst)
    }
}

/// Errors that can occur during audio capture
#[derive(Debug, thiserror::Error)]
pub enum AudioCaptureError {
    #[error("No audio input device found")]
    NoInputDevice,

    #[error("No supported audio configuration found")]
    NoSupportedConfig,

    #[error("Audio configuration error: {0}")]
    ConfigError(String),

    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    #[error("Audio device error: {0}")]
    DeviceError(#[from] cpal::DevicesError),

    #[error("Audio stream error: {0}")]
    StreamError(#[from] cpal::BuildStreamError),

    #[error("Audio play error: {0}")]
    PlayError(#[from] cpal::PlayStreamError),

    #[error("Default config error: {0}")]
    DefaultConfigError(#[from] cpal::DefaultStreamConfigError),
}
