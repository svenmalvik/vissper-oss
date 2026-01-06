//! Transcript event handling
//!
//! Handles events from the transcription service and updates the UI accordingly.

use crate::transcription::{TranscriptEvent, TranscriptionSession};
use crate::transcription_window;
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

/// Event handler loop that processes transcription events
#[tracing::instrument(skip(event_rx, session_data))]
async fn run_event_handler(
    mut event_rx: tokio::sync::broadcast::Receiver<TranscriptEvent>,
    session_data: Arc<Mutex<TranscriptionSession>>,
    log_events: bool,
) {
    while let Ok(event) = event_rx.recv().await {
        handle_transcript_event(&event, &session_data, log_events);
    }
}

/// Spawn the event handler task for processing transcription events
pub(super) fn spawn_event_handler(
    event_rx: tokio::sync::broadcast::Receiver<TranscriptEvent>,
    session_data: Arc<Mutex<TranscriptionSession>>,
    log_events: bool,
) {
    tokio::spawn(run_event_handler(event_rx, session_data, log_events));
}

/// Handle a single transcript event
fn handle_transcript_event(
    event: &TranscriptEvent,
    session_data: &Arc<Mutex<TranscriptionSession>>,
    log_events: bool,
) {
    match event {
        TranscriptEvent::PartialTranscript { ref text } => {
            if log_events {
                info!("Partial: {}", text);
            }
            let committed = get_committed_transcript(session_data);
            // Update the live tab with the transcript
            transcription_window::TranscriptionWindow::update_live_text(&committed, Some(text));
        }
        TranscriptEvent::CommittedTranscript { ref text } => {
            if log_events {
                info!("Committed: {}", text);
            }
            let committed = get_committed_transcript(session_data);
            // Update the live tab with the committed transcript
            transcription_window::TranscriptionWindow::update_live_text(&committed, None);
        }
        TranscriptEvent::Error { ref message } => {
            error!("Transcription error: {}", message);
        }
        TranscriptEvent::ConnectionLost => {
            handle_connection_lost(session_data, log_events);
        }
        TranscriptEvent::Reconnecting { attempt } => {
            if log_events {
                info!("Reconnecting to STT service (attempt {})", attempt);
            }
        }
        TranscriptEvent::Reconnected => {
            if log_events {
                info!("Reconnected to STT service");
            }
            let committed = get_committed_transcript(session_data);
            transcription_window::TranscriptionWindow::update_live_text(&committed, None);
        }
        TranscriptEvent::ReconnectFailed => {
            error!("Failed to reconnect to STT service after multiple attempts");
        }
    }
}

/// Handle connection lost event
fn handle_connection_lost(session_data: &Arc<Mutex<TranscriptionSession>>, log_events: bool) {
    if log_events {
        warn!("Connection to STT service lost");
    }
    // Only update UI if recording wasn't manually stopped
    if let Ok(session) = session_data.lock() {
        if !session.manually_stopped {
            let committed = session.full_transcript();
            drop(session);
            transcription_window::TranscriptionWindow::update_live_text(&committed, None);
        }
    }
}

/// Get committed transcript from session
pub(super) fn get_committed_transcript(session_data: &Arc<Mutex<TranscriptionSession>>) -> String {
    if let Ok(session) = session_data.lock() {
        session.full_transcript()
    } else {
        String::new()
    }
}
