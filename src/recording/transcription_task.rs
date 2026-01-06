//! Transcription task management
//!
//! Handles the async transcription task including WebSocket connection
//! to Azure OpenAI or OpenAI Realtime API and error handling.

use crate::audio::AudioChunk;
use crate::menubar;
use crate::transcription::TranscriptionClient;
use crate::transcription_window;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{error, info};

use super::RecordingSession;

/// Provider-specific configuration for transcription
pub(super) enum TranscriptionProviderConfig {
    Azure {
        endpoint: String,
        deployment: String,
        api_key: String,
    },
    OpenAI {
        api_key: String,
    },
}

/// Configuration for starting a transcription task
pub(super) struct TranscriptionTaskConfig {
    pub transcription_client: TranscriptionClient,
    pub provider_config: TranscriptionProviderConfig,
    pub audio_rx: mpsc::Receiver<AudioChunk>,
    pub recording_state: Arc<Mutex<Option<RecordingSession>>>,
}

/// Run the transcription task with error handling
#[tracing::instrument(skip(config))]
async fn run_transcription_task(config: TranscriptionTaskConfig) {
    // Start transcription via the appropriate provider
    let transcription_result = match &config.provider_config {
        TranscriptionProviderConfig::Azure {
            endpoint,
            deployment,
            api_key,
        } => {
            info!("Starting Azure OpenAI Realtime transcription");
            config
                .transcription_client
                .start_azure(endpoint, deployment, api_key, config.audio_rx)
                .await
        }
        TranscriptionProviderConfig::OpenAI { api_key } => {
            info!("Starting OpenAI Realtime transcription");
            config
                .transcription_client
                .start_openai(api_key, config.audio_rx)
                .await
        }
    };

    // Get final transcript and check if manually stopped
    let session = config.transcription_client.session();
    let transcript = session.full_transcript();
    let manually_stopped = session.manually_stopped;

    // Handle transcription errors - stop audio capture immediately
    if let Err(ref e) = transcription_result {
        error!("Transcription error: {}", e);

        // Stop audio capture to prevent buffer overflow warnings
        if let Ok(mut state) = config.recording_state.lock() {
            if let Some(ref mut recording_session) = *state {
                recording_session.audio_handle.stop();
            }
        }

        // Update UI to show connection failed
        if !manually_stopped {
            menubar::MenuBar::set_recording(false);
            transcription_window::TranscriptionWindow::set_recording_state(false);

            let error_message = format!("{}", e);
            if transcript.trim().is_empty() {
                transcription_window::TranscriptionWindow::update_live_text(
                    &format!("Connection failed\n\n{}", error_message),
                    None,
                );
            } else {
                transcription_window::TranscriptionWindow::update_live_text(
                    &format!("{}\n\nConnection lost: {}", transcript, error_message),
                    None,
                );
                transcription_window::TranscriptionWindow::show_save_button(transcript.clone());
            }
        }
    } else if !manually_stopped && !transcript.trim().is_empty() {
        // Show save button if NOT manually stopped and transcript has content
        transcription_window::TranscriptionWindow::show_save_button(transcript);
    }

    // Clear recording state
    if let Ok(mut state) = config.recording_state.lock() {
        *state = None;
    }
}

/// Spawn the transcription task
pub(super) fn spawn_transcription_task(config: TranscriptionTaskConfig) {
    tokio::spawn(run_transcription_task(config));
}
