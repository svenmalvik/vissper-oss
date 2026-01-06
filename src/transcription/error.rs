//! Error types for transcription module

/// WebSocket connection timeout in seconds
pub(super) const WS_CONNECT_TIMEOUT_SECS: u64 = 30;

/// Errors that can occur during transcription
#[derive(Debug, thiserror::Error)]
pub enum TranscriptionError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Connection timeout - Azure did not respond within {WS_CONNECT_TIMEOUT_SECS} seconds")]
    ConnectionTimeout,
}
