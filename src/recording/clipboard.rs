//! Clipboard functionality for recording module
//!
//! Handles copying transcripts to the system clipboard.

use arboard::Clipboard;
use tracing::{error, info};

/// Copy text to clipboard
pub(crate) fn copy_to_clipboard(transcript: &str) {
    if !transcript.trim().is_empty() {
        match Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(transcript) {
                Ok(_) => {
                    info!(
                        "Transcript copied to clipboard ({} chars)",
                        transcript.len()
                    );
                }
                Err(e) => {
                    error!("Failed to copy transcript to clipboard: {}", e);
                }
            },
            Err(e) => {
                error!("Failed to initialize clipboard: {}", e);
            }
        }
    } else {
        info!("No transcript to copy (empty)");
    }
}
