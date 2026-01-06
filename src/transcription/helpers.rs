//! Helper functions for transcription module

use base64::Engine;

/// Generate a random WebSocket key
pub(super) fn generate_ws_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut key = [0u8; 16];
    rng.fill(&mut key);
    base64::engine::general_purpose::STANDARD.encode(key)
}
