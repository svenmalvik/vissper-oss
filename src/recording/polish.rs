//! Transcript polishing functionality
//!
//! Handles async transcript polishing via Azure OpenAI or OpenAI.
//! Users provide their own credentials for their selected provider.

use crate::azure_openai::AzureOpenAIClient;
use crate::error::ResponseError;
use crate::keychain;
use crate::openai::OpenAIClient;
use crate::preferences::{self, AiProvider};
use crate::response::PolishConfig;
use crate::transcription_window::{self, TabType};
use tokio::time::{timeout, Duration};
use tracing::{error, info};

use super::polish_helpers::{
    handle_polish_error, handle_polish_failure, handle_polish_success, handle_transcript_too_large,
    reset_processing_state,
};

/// Timeout for polish API calls (2 minutes for long transcripts)
const POLISH_TIMEOUT: Duration = Duration::from_secs(120);

/// Execute polish via Azure OpenAI connection
async fn azure_polish(transcript: &str, config: &PolishConfig, target_tab: TabType) {
    // Get Azure credentials
    let creds = match keychain::get_azure_credentials() {
        Ok(c) => c,
        Err(e) => {
            error!("Azure credentials not found: {}", e);
            handle_polish_failure(transcript, target_tab);
            return;
        }
    };

    info!(
        endpoint = %creds.endpoint_url,
        deployment = %creds.polish_deployment,
        "Polishing transcript via Azure OpenAI"
    );

    let client = match AzureOpenAIClient::new(&creds) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create Azure client: {}", e);
            handle_polish_failure(transcript, target_tab);
            return;
        }
    };

    let polish_result = timeout(POLISH_TIMEOUT, client.polish_transcript(transcript, config)).await;

    match polish_result {
        Err(_) => {
            error!("Azure polish request timed out after {:?}", POLISH_TIMEOUT);
            handle_polish_failure(transcript, target_tab);
        }
        Ok(Ok(polished)) => {
            info!(
                "Transcript polished via Azure ({} -> {} chars)",
                transcript.len(),
                polished.len()
            );
            handle_polish_success(polished, target_tab);
        }
        Ok(Err(ResponseError::TranscriptTooLarge { length, max_length })) => {
            error!(
                "Transcript too large for Azure: {} chars (max: {})",
                length, max_length
            );
            handle_transcript_too_large(transcript, length, max_length, target_tab);
        }
        Ok(Err(e)) => {
            error!("Failed to polish transcript via Azure: {}", e);
            handle_polish_error(transcript, target_tab);
        }
    }
}

/// Execute polish via OpenAI connection
async fn openai_polish(transcript: &str, config: &PolishConfig, target_tab: TabType) {
    // Get OpenAI credentials
    let creds = match keychain::get_openai_credentials() {
        Ok(c) => c,
        Err(e) => {
            error!("OpenAI credentials not found: {}", e);
            handle_polish_failure(transcript, target_tab);
            return;
        }
    };

    info!("Polishing transcript via OpenAI (gpt-5.2)");

    let client = match OpenAIClient::new(&creds) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create OpenAI client: {}", e);
            handle_polish_failure(transcript, target_tab);
            return;
        }
    };

    let polish_result = timeout(POLISH_TIMEOUT, client.polish_transcript(transcript, config)).await;

    match polish_result {
        Err(_) => {
            error!("OpenAI polish request timed out after {:?}", POLISH_TIMEOUT);
            handle_polish_failure(transcript, target_tab);
        }
        Ok(Ok(polished)) => {
            info!(
                "Transcript polished via OpenAI ({} -> {} chars)",
                transcript.len(),
                polished.len()
            );
            handle_polish_success(polished, target_tab);
        }
        Ok(Err(ResponseError::TranscriptTooLarge { length, max_length })) => {
            error!(
                "Transcript too large for OpenAI: {} chars (max: {})",
                length, max_length
            );
            handle_transcript_too_large(transcript, length, max_length, target_tab);
        }
        Ok(Err(e)) => {
            error!("Failed to polish transcript via OpenAI: {}", e);
            handle_polish_error(transcript, target_tab);
        }
    }
}

/// Polish transcript using the selected provider
async fn polish_with_provider(transcript: &str, config: &PolishConfig, target_tab: TabType) {
    let provider = preferences::get_ai_provider();

    match provider {
        AiProvider::Azure => {
            azure_polish(transcript, config, target_tab).await;
        }
        AiProvider::OpenAI => {
            openai_polish(transcript, config, target_tab).await;
        }
    }
}

/// Async function to polish transcript (called when stopping recording)
#[tracing::instrument(skip(transcript))]
pub(super) async fn polish_transcript_async(transcript: String, config: PolishConfig) {
    // Determine target tab based on config
    let target_tab = if config.prompt_type.as_deref() == Some("live_meeting") {
        TabType::MeetingNotes
    } else {
        TabType::BasicPolish
    };

    // If transcript is empty, skip polishing
    if transcript.trim().is_empty() {
        info!("No transcript to polish (empty)");
        reset_processing_state();
        return;
    }

    // Store the raw transcript in the live tab
    transcription_window::TranscriptionWindow::update_live_text(&transcript, None);

    // Polish via selected provider
    polish_with_provider(&transcript, &config, target_tab).await;

    reset_processing_state();
}

/// Async function to polish transcript on-demand (called when clicking empty tab)
#[tracing::instrument(skip(transcript))]
pub(crate) async fn polish_transcript_on_demand(transcript: String, target_tab: TabType) {
    // Determine config based on target tab
    let config = match target_tab {
        TabType::MeetingNotes => PolishConfig::live_meeting(),
        TabType::BasicPolish => PolishConfig::basic_polish(),
        TabType::Live => return,
    };

    if transcript.trim().is_empty() {
        info!("No transcript to polish on-demand (empty)");
        reset_processing_state();
        return;
    }

    // Polish via selected provider
    polish_with_provider(&transcript, &config, target_tab).await;

    reset_processing_state();
}
