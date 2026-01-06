use thiserror::Error;

/// Application-level errors
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Keychain error: {0}")]
    Keychain(#[from] KeychainError),
}

/// Response/Polish-related errors
#[derive(Debug, Error)]
pub enum ResponseError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Invalid response from server: {0}")]
    InvalidResponse(String),

    #[error("Server error ({status}): {message}")]
    ServerError { status: u16, message: String },

    #[error("Transcript too large: {length} characters (max: {max_length})")]
    TranscriptTooLarge { length: usize, max_length: usize },
}

/// Keychain-related errors
#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("Failed to store credentials: {0}")]
    Store(String),

    #[error("Failed to retrieve credentials: {0}")]
    Retrieve(String),

    #[error("Failed to delete credentials: {0}")]
    Delete(String),

    #[error("Invalid credential data: {0}")]
    InvalidData(String),

    #[error("Credential storage not implemented for this platform")]
    NotImplemented,
}
