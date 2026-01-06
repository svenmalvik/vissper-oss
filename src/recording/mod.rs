//! Recording session management module
//!
//! Provides shared logic for starting and stopping recording sessions,
//! used by both menu bar and transcription window callbacks.
//!
//! # Architecture
//! A recording session consists of:
//! - Audio capture from the microphone
//! - Azure OpenAI or OpenAI Realtime STT connection
//! - Real-time transcription
//! - UI updates in the transcription window
//! - Transcript polishing via the selected provider (on stop)
//!
//! The session state is shared via `Arc<Mutex<Option<RecordingSession>>>`.

mod clipboard;
mod events;
mod polish;
mod polish_helpers;
mod transcription_task;

// Re-export polish_transcript_on_demand for use from main.rs
pub(crate) use polish::polish_transcript_on_demand;

use crate::audio::{self, AudioCaptureHandle, AZURE_SAMPLE_RATE, OPENAI_SAMPLE_RATE};
use crate::keychain;
use crate::menubar;
use crate::preferences::{self, AiProvider};
use crate::response::PolishConfig;
use crate::transcription::{self, TranscriptionSession};
use crate::transcription_window;
use std::sync::{Arc, Mutex};
use tracing::{error, info};

use transcription_task::{
    spawn_transcription_task, TranscriptionProviderConfig, TranscriptionTaskConfig,
};

/// Holds the state of an active recording session
///
/// Contains the audio capture handle and shared transcription session data.
/// Both must be kept alive for the duration of the recording.
pub(crate) struct RecordingSession {
    /// Handle to control audio capture (stop, check status)
    pub(crate) audio_handle: AudioCaptureHandle,
    /// Shared session data containing transcripts and timestamps
    pub(crate) session_data: Arc<Mutex<TranscriptionSession>>,
}

/// Start a recording session
///
/// This function:
/// 1. Gets credentials from keychain based on selected provider
/// 2. Starts audio capture with provider-specific sample rate
/// 3. Creates transcription client for the selected provider
/// 4. Spawns event handler for UI updates
/// 5. Spawns transcription task
/// 6. Updates recording state and UI
pub(crate) fn start_recording(
    recording_state: Arc<Mutex<Option<RecordingSession>>>,
    log_events: bool,
) {
    // Determine which provider to use
    let provider = preferences::get_ai_provider();
    info!("Starting recording with provider: {:?}", provider);

    // Get credentials and create provider config based on selected provider
    let (provider_config, sample_rate) = match provider {
        AiProvider::Azure => match keychain::get_azure_credentials() {
            Ok(creds) => (
                TranscriptionProviderConfig::Azure {
                    endpoint: creds.endpoint_url,
                    deployment: creds.stt_deployment,
                    api_key: creds.api_key,
                },
                AZURE_SAMPLE_RATE,
            ),
            Err(e) => {
                error!("Cannot start recording without Azure credentials: {}", e);
                transcription_window::TranscriptionWindow::show();
                transcription_window::TranscriptionWindow::update_live_text(
                        "Azure credentials not configured.\n\nPlease go to Settings and enter your Azure OpenAI credentials.",
                        None,
                    );
                return;
            }
        },
        AiProvider::OpenAI => match keychain::get_openai_credentials() {
            Ok(creds) => (
                TranscriptionProviderConfig::OpenAI {
                    api_key: creds.api_key,
                },
                OPENAI_SAMPLE_RATE,
            ),
            Err(e) => {
                error!("Cannot start recording without OpenAI credentials: {}", e);
                transcription_window::TranscriptionWindow::show();
                transcription_window::TranscriptionWindow::update_live_text(
                        "OpenAI credentials not configured.\n\nPlease go to Settings and enter your OpenAI API key.",
                        None,
                    );
                return;
            }
        },
    };

    // Start audio capture with provider-specific sample rate
    let (audio_handle, audio_rx) = match audio::start_capture_with_sample_rate(sample_rate) {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to start audio capture: {}", e);
            return;
        }
    };

    // Get language preference
    let language_code = preferences::get_language_code();
    info!(
        "Starting transcription with language: {}, sample_rate: {}Hz",
        language_code, sample_rate
    );

    // Create transcription client based on provider
    let transcription_client = match provider {
        AiProvider::Azure => transcription::TranscriptionClient::new_azure(language_code),
        AiProvider::OpenAI => transcription::TranscriptionClient::new_openai(language_code),
    };

    // Get the session Arc for sharing
    let session_data = transcription_client.session_arc();

    // Subscribe to events for logging and UI updates
    let event_rx = transcription_client.subscribe();

    // Clone for tasks
    let recording_state_store = recording_state.clone();
    let session_data_for_events = session_data.clone();

    // Show transcription overlay window and set recording state immediately
    transcription_window::TranscriptionWindow::show();
    transcription_window::TranscriptionWindow::reset_tabs();
    transcription_window::TranscriptionWindow::set_recording_state(true);
    transcription_window::TranscriptionWindow::set_recording_type();
    transcription_window::TranscriptionWindow::update_live_text("", Some("Listening..."));
    transcription_window::TranscriptionWindow::hide_save_button();

    // Spawn event handler
    events::spawn_event_handler(event_rx, session_data_for_events, log_events);

    // Spawn transcription task
    spawn_transcription_task(TranscriptionTaskConfig {
        transcription_client,
        provider_config,
        audio_rx,
        recording_state: recording_state_store,
    });

    // Store the audio handle and session data
    if let Ok(mut state) = recording_state.lock() {
        *state = Some(RecordingSession {
            audio_handle,
            session_data: session_data.clone(),
        });
    }

    menubar::MenuBar::set_recording(true);
    info!("Recording started with {:?} provider", provider);
}

/// Stop a recording session without polishing (raw transcript)
pub(crate) fn stop_recording_no_polish(recording_state: Arc<Mutex<Option<RecordingSession>>>) {
    let transcript = get_full_transcript(&recording_state);
    stop_audio_capture(&recording_state);

    // Update UI - recording stopped
    menubar::MenuBar::set_recording(false);
    transcription_window::TranscriptionWindow::set_recording_state(false);
    transcription_window::TranscriptionWindow::update_live_text(&transcript, None);
    info!("Recording stopped (no polishing)");

    // Copy raw transcript to clipboard
    clipboard::copy_to_clipboard(&transcript);

    // Show save button if transcript is not empty
    if !transcript.trim().is_empty() {
        transcription_window::TranscriptionWindow::show_save_button(transcript);
    }
}

/// Stop a recording session and polish the transcript (Basic polishing mode)
pub(crate) fn stop_recording(recording_state: Arc<Mutex<Option<RecordingSession>>>) {
    stop_recording_with_config(recording_state, PolishConfig::basic_polish());
}

/// Stop a recording session with Live Meeting polishing
pub(crate) fn stop_live_meeting_recording(recording_state: Arc<Mutex<Option<RecordingSession>>>) {
    stop_recording_with_config(recording_state, PolishConfig::live_meeting());
}

/// Internal function to stop recording with a specific polish config
fn stop_recording_with_config(
    recording_state: Arc<Mutex<Option<RecordingSession>>>,
    config: PolishConfig,
) {
    let transcript = get_full_transcript(&recording_state);
    stop_audio_capture(&recording_state);

    // Update UI - recording stopped, processing started
    menubar::MenuBar::set_recording(false);
    menubar::MenuBar::set_processing(true);
    transcription_window::TranscriptionWindow::set_recording_state(false);
    transcription_window::TranscriptionWindow::set_processing_state(true);
    transcription_window::TranscriptionWindow::update_live_text(&transcript, Some("Polishing..."));
    info!("Recording stopped, polishing transcript...");

    // Spawn async task to polish the transcript
    tokio::spawn(async move {
        polish::polish_transcript_async(transcript, config).await;
    });
}

/// Stop audio capture and mark session as manually stopped
fn stop_audio_capture(recording_state: &Arc<Mutex<Option<RecordingSession>>>) {
    if let Ok(mut state) = recording_state.lock() {
        if let Some(ref mut session) = *state {
            if let Ok(mut session_data) = session.session_data.lock() {
                session_data.manually_stopped = true;
            }
            session.audio_handle.stop();
        }
    }
}

/// Get full transcript including partial text
pub(crate) fn get_full_transcript(
    recording_state: &Arc<Mutex<Option<RecordingSession>>>,
) -> String {
    let Ok(state) = recording_state.lock() else {
        return String::new();
    };

    if let Some(ref recording_session) = *state {
        let Ok(session) = recording_session.session_data.lock() else {
            return String::new();
        };

        let committed = session.full_transcript();
        if let Some(ref partial) = session.partial_transcript {
            if !partial.trim().is_empty() {
                if committed.is_empty() {
                    return partial.clone();
                } else {
                    return format!("{} {}", committed, partial);
                }
            }
        }
        committed
    } else {
        String::new()
    }
}
