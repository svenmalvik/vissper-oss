//! Transcription module for real-time speech-to-text
//!
//! Handles WebSocket connection to Azure OpenAI or OpenAI Realtime API
//! for STT using GPT-4o Transcribe. Includes automatic reconnection on connection loss.

mod azure_connection;
mod azure_messages;
mod error;
mod helpers;
mod openai_connection;
mod openai_messages;
mod session;

pub use error::TranscriptionError;
pub use session::TranscriptionSession;

use crate::audio::AudioChunk;
use futures_util::StreamExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{sleep, timeout};
use tokio_tungstenite::connect_async;
use tracing::{error, info, warn};

/// Transcript event for subscribers
#[derive(Clone, Debug)]
pub enum TranscriptEvent {
    /// Partial transcript (still being recognized)
    PartialTranscript { text: String },
    /// Final committed transcript segment
    CommittedTranscript { text: String },
    /// Transcription error
    Error { message: String },
    /// Connection was lost
    ConnectionLost,
    /// Attempting to reconnect
    Reconnecting { attempt: u32 },
    /// Successfully reconnected
    Reconnected,
    /// Failed to reconnect after max attempts
    ReconnectFailed,
}

/// Maximum number of reconnection attempts
const MAX_RECONNECT_ATTEMPTS: u32 = 5;

/// Delay between reconnection attempts in seconds
const RECONNECT_DELAY_SECS: u64 = 2;

/// Transcription client for managing Azure STT sessions
pub struct TranscriptionClient {
    language_code: String,
    session: Arc<Mutex<TranscriptionSession>>,
    event_tx: broadcast::Sender<TranscriptEvent>,
    should_stop: Arc<AtomicBool>,
}

impl TranscriptionClient {
    /// Create a new transcription client for Azure OpenAI
    ///
    /// # Arguments
    /// * `language_code` - Language code for transcription (e.g., "en", "no", "da", "fi", "de")
    pub fn new_azure(language_code: String) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            language_code,
            session: Arc::new(Mutex::new(TranscriptionSession::default())),
            event_tx,
            should_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a new transcription client for OpenAI
    ///
    /// # Arguments
    /// * `language_code` - Language code for transcription (e.g., "en", "no", "da", "fi", "de")
    pub fn new_openai(language_code: String) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            language_code,
            session: Arc::new(Mutex::new(TranscriptionSession::default())),
            event_tx,
            should_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Subscribe to transcript events
    pub fn subscribe(&self) -> broadcast::Receiver<TranscriptEvent> {
        self.event_tx.subscribe()
    }

    /// Get the current session data
    ///
    /// Returns the session data or a default if the mutex is poisoned.
    pub fn session(&self) -> TranscriptionSession {
        match self.session.lock() {
            Ok(session) => session.clone(),
            Err(poisoned) => {
                warn!("Session mutex was poisoned, recovering data");
                poisoned.into_inner().clone()
            }
        }
    }

    /// Get a reference to the session Arc for sharing
    pub fn session_arc(&self) -> Arc<Mutex<TranscriptionSession>> {
        self.session.clone()
    }

    /// Start an Azure OpenAI Realtime transcription session
    ///
    /// Connects directly to Azure OpenAI Realtime API for STT using GPT-4o Transcribe.
    ///
    /// # Arguments
    /// * `endpoint_url` - Azure OpenAI endpoint URL (e.g., "https://myresource.openai.azure.com")
    /// * `stt_deployment` - Deployment name for STT (e.g., "gpt-4o-transcribe")
    /// * `api_key` - Azure API key
    /// * `audio_rx` - Receiver for audio chunks from the capture module
    pub async fn start_azure(
        &self,
        endpoint_url: &str,
        stt_deployment: &str,
        api_key: &str,
        mut audio_rx: mpsc::Receiver<AudioChunk>,
    ) -> Result<(), TranscriptionError> {
        use azure_connection::{
            build_azure_ws_request, build_azure_ws_url, resend_azure_buffered_chunks,
            send_session_init, spawn_azure_receive_task, spawn_azure_send_task,
        };

        // Build Azure WebSocket URL
        let ws_url = build_azure_ws_url(endpoint_url, stt_deployment);

        info!(
            endpoint_url = %endpoint_url,
            stt_deployment = %stt_deployment,
            ws_url = %ws_url,
            language_code = %self.language_code,
            "Connecting to Azure OpenAI Realtime for STT"
        );

        let parsed_url = url::Url::parse(&ws_url)
            .map_err(|e| TranscriptionError::ConnectionError(e.to_string()))?;
        let host = parsed_url
            .host_str()
            .ok_or_else(|| TranscriptionError::ConnectionError("Invalid URL: no host".to_string()))?
            .to_string();

        // Create internal audio buffer channel for reconnection support
        let (audio_buffer_tx, mut audio_buffer_rx) = mpsc::channel::<AudioChunk>(1000);

        let session = self.session.clone();
        let event_tx = self.event_tx.clone();
        let should_stop = self.should_stop.clone();
        let language_code = self.language_code.clone();

        // Forward audio from external channel to internal buffer
        let should_stop_forwarder = should_stop.clone();
        let audio_forwarder = tokio::spawn(async move {
            let mut chunk_count = 0u64;
            info!("Azure audio forwarder started");
            while let Some(chunk) = audio_rx.recv().await {
                chunk_count += 1;
                if chunk_count == 1 || chunk_count.is_multiple_of(100) {
                    info!(
                        "Azure audio forwarder: received chunk #{}, {} samples",
                        chunk_count,
                        chunk.samples.len()
                    );
                }
                if should_stop_forwarder.load(Ordering::SeqCst) {
                    info!("Azure audio forwarder: stopping (should_stop flag)");
                    break;
                }
                if audio_buffer_tx.send(chunk).await.is_err() {
                    info!("Azure audio forwarder: buffer channel closed");
                    break;
                }
            }
            info!("Azure audio forwarder exiting after {} chunks", chunk_count);
        });

        // Main connection loop with reconnection support
        let mut reconnect_attempts = 0u32;
        let mut is_first_connection = true;
        let mut pending_chunks: Vec<AudioChunk> = Vec::new();

        loop {
            if should_stop.load(Ordering::SeqCst) {
                info!("Azure transcription stopped by user");
                break;
            }

            // Handle reconnection logic
            if !is_first_connection {
                reconnect_attempts += 1;
                if reconnect_attempts > MAX_RECONNECT_ATTEMPTS {
                    error!(
                        "Failed to reconnect to Azure after {} attempts",
                        MAX_RECONNECT_ATTEMPTS
                    );
                    let _ = event_tx.send(TranscriptEvent::ReconnectFailed);
                    break;
                }
                info!(
                    "Reconnecting to Azure STT (attempt {}/{})",
                    reconnect_attempts, MAX_RECONNECT_ATTEMPTS
                );
                let _ = event_tx.send(TranscriptEvent::Reconnecting {
                    attempt: reconnect_attempts,
                });
                sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
            } else {
                info!("Connecting to Azure STT: {}", ws_url);
            }

            // Build WebSocket request with Azure auth
            let request = match build_azure_ws_request(&ws_url, &host, api_key) {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to build Azure WebSocket request: {}", e);
                    if is_first_connection {
                        return Err(TranscriptionError::ConnectionError(e));
                    }
                    is_first_connection = false;
                    continue;
                }
            };

            // Attempt connection with timeout
            let ws_result = timeout(
                Duration::from_secs(error::WS_CONNECT_TIMEOUT_SECS),
                connect_async(request),
            )
            .await;

            let ws_stream = match ws_result {
                Ok(Ok((stream, _response))) => stream,
                Ok(Err(e)) => {
                    error!("Azure WebSocket connection failed: {}", e);
                    if is_first_connection {
                        return Err(TranscriptionError::ConnectionError(e.to_string()));
                    }
                    let _ = event_tx.send(TranscriptEvent::ConnectionLost);
                    is_first_connection = false;
                    continue;
                }
                Err(_) => {
                    error!("Azure WebSocket connection timed out");
                    if is_first_connection {
                        return Err(TranscriptionError::ConnectionTimeout);
                    }
                    let _ = event_tx.send(TranscriptEvent::ConnectionLost);
                    is_first_connection = false;
                    continue;
                }
            };

            info!("Connected to Azure OpenAI Realtime");

            if !is_first_connection {
                let _ = event_tx.send(TranscriptEvent::Reconnected);
                reconnect_attempts = 0;
            }
            is_first_connection = false;

            let (mut ws_sink, ws_stream) = ws_stream.split();

            // Send session initialization
            let language = if self.language_code.is_empty() {
                None
            } else {
                Some(language_code.as_str())
            };
            if let Err(e) = send_session_init(&mut ws_sink, stt_deployment, language).await {
                error!("Failed to send Azure session init: {}", e);
                let _ = event_tx.send(TranscriptEvent::ConnectionLost);
                continue;
            }

            // Resend buffered audio chunks
            if resend_azure_buffered_chunks(&mut ws_sink, &mut pending_chunks)
                .await
                .is_err()
            {
                continue; // Reconnect
            }

            // Channel to signal connection failure
            let (connection_lost_tx, connection_lost_rx) = mpsc::channel::<()>(1);

            // Spawn receive and send tasks
            let recv_task = spawn_azure_receive_task(
                ws_stream,
                session.clone(),
                event_tx.clone(),
                should_stop.clone(),
            );

            let send_task = spawn_azure_send_task(
                ws_sink,
                audio_buffer_rx,
                connection_lost_rx,
                should_stop.clone(),
            );

            // Wait for receive task
            let recv_result = recv_task
                .await
                .unwrap_or(azure_connection::AzureReceiveResult {
                    connection_ok: false,
                    quota_exceeded: false,
                });

            // Signal send task
            let _ = connection_lost_tx.send(()).await;

            // Get results from send task
            let send_result = send_task
                .await
                .unwrap_or(azure_connection::AzureSendResult {
                    audio_rx: mpsc::channel::<AudioChunk>(1).1,
                    pending_chunks: Vec::new(),
                    stopped_by_user: true,
                });

            audio_buffer_rx = send_result.audio_rx;
            pending_chunks = send_result.pending_chunks;

            // Check if we should stop
            if should_stop.load(Ordering::SeqCst) || send_result.stopped_by_user {
                info!("Azure transcription session ended");
                break;
            }

            if recv_result.quota_exceeded {
                info!("Azure quota exceeded - stopping transcription");
                break;
            }

            if recv_result.connection_ok {
                info!("Azure connection closed normally");
                break;
            }

            warn!("Azure connection lost, will attempt to reconnect...");
        }

        let _ = audio_forwarder.await;
        Ok(())
    }

    /// Start an OpenAI Realtime transcription session
    ///
    /// Connects directly to OpenAI Realtime API for STT using gpt-4o-transcribe.
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key
    /// * `audio_rx` - Receiver for audio chunks from the capture module
    pub async fn start_openai(
        &self,
        api_key: &str,
        mut audio_rx: mpsc::Receiver<AudioChunk>,
    ) -> Result<(), TranscriptionError> {
        use openai_connection::{
            build_openai_ws_request, build_openai_ws_url, resend_openai_buffered_chunks,
            send_session_init, spawn_openai_receive_task, spawn_openai_send_task,
        };

        // Build OpenAI WebSocket URL
        let ws_url = build_openai_ws_url();

        info!(
            ws_url = %ws_url,
            language_code = %self.language_code,
            "Connecting to OpenAI Realtime for STT"
        );

        // Create internal audio buffer channel for reconnection support
        let (audio_buffer_tx, mut audio_buffer_rx) = mpsc::channel::<AudioChunk>(1000);

        let session = self.session.clone();
        let event_tx = self.event_tx.clone();
        let should_stop = self.should_stop.clone();
        let language_code = self.language_code.clone();

        // Forward audio from external channel to internal buffer
        let should_stop_forwarder = should_stop.clone();
        let audio_forwarder = tokio::spawn(async move {
            let mut chunk_count = 0u64;
            info!("OpenAI audio forwarder started");
            while let Some(chunk) = audio_rx.recv().await {
                chunk_count += 1;
                if chunk_count == 1 || chunk_count.is_multiple_of(100) {
                    info!(
                        "OpenAI audio forwarder: received chunk #{}, {} samples",
                        chunk_count,
                        chunk.samples.len()
                    );
                }
                if should_stop_forwarder.load(Ordering::SeqCst) {
                    info!("OpenAI audio forwarder: stopping (should_stop flag)");
                    break;
                }
                if audio_buffer_tx.send(chunk).await.is_err() {
                    info!("OpenAI audio forwarder: buffer channel closed");
                    break;
                }
            }
            info!(
                "OpenAI audio forwarder exiting after {} chunks",
                chunk_count
            );
        });

        // Main connection loop with reconnection support
        let mut reconnect_attempts = 0u32;
        let mut is_first_connection = true;
        let mut pending_chunks: Vec<AudioChunk> = Vec::new();

        loop {
            if should_stop.load(Ordering::SeqCst) {
                info!("OpenAI transcription stopped by user");
                break;
            }

            // Handle reconnection logic
            if !is_first_connection {
                reconnect_attempts += 1;
                if reconnect_attempts > MAX_RECONNECT_ATTEMPTS {
                    error!(
                        "Failed to reconnect to OpenAI after {} attempts",
                        MAX_RECONNECT_ATTEMPTS
                    );
                    let _ = event_tx.send(TranscriptEvent::ReconnectFailed);
                    break;
                }
                info!(
                    "Reconnecting to OpenAI STT (attempt {}/{})",
                    reconnect_attempts, MAX_RECONNECT_ATTEMPTS
                );
                let _ = event_tx.send(TranscriptEvent::Reconnecting {
                    attempt: reconnect_attempts,
                });
                sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
            } else {
                info!("Connecting to OpenAI STT: {}", ws_url);
            }

            // Build WebSocket request with OpenAI auth
            let request = match build_openai_ws_request(&ws_url, api_key) {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to build OpenAI WebSocket request: {}", e);
                    if is_first_connection {
                        return Err(TranscriptionError::ConnectionError(e));
                    }
                    is_first_connection = false;
                    continue;
                }
            };

            // Attempt connection with timeout
            let ws_result = timeout(
                Duration::from_secs(error::WS_CONNECT_TIMEOUT_SECS),
                connect_async(request),
            )
            .await;

            let ws_stream = match ws_result {
                Ok(Ok((stream, _response))) => stream,
                Ok(Err(e)) => {
                    error!("OpenAI WebSocket connection failed: {}", e);
                    if is_first_connection {
                        return Err(TranscriptionError::ConnectionError(e.to_string()));
                    }
                    let _ = event_tx.send(TranscriptEvent::ConnectionLost);
                    is_first_connection = false;
                    continue;
                }
                Err(_) => {
                    error!("OpenAI WebSocket connection timed out");
                    if is_first_connection {
                        return Err(TranscriptionError::ConnectionTimeout);
                    }
                    let _ = event_tx.send(TranscriptEvent::ConnectionLost);
                    is_first_connection = false;
                    continue;
                }
            };

            info!("Connected to OpenAI Realtime");

            if !is_first_connection {
                let _ = event_tx.send(TranscriptEvent::Reconnected);
                reconnect_attempts = 0;
            }
            is_first_connection = false;

            let (mut ws_sink, ws_stream) = ws_stream.split();

            // Send session initialization
            let language = if self.language_code.is_empty() {
                None
            } else {
                Some(language_code.as_str())
            };
            if let Err(e) = send_session_init(&mut ws_sink, language).await {
                error!("Failed to send OpenAI session init: {}", e);
                let _ = event_tx.send(TranscriptEvent::ConnectionLost);
                continue;
            }

            // Resend buffered audio chunks
            if resend_openai_buffered_chunks(&mut ws_sink, &mut pending_chunks)
                .await
                .is_err()
            {
                continue; // Reconnect
            }

            // Channel to signal connection failure
            let (connection_lost_tx, connection_lost_rx) = mpsc::channel::<()>(1);

            // Spawn receive and send tasks
            let recv_task = spawn_openai_receive_task(
                ws_stream,
                session.clone(),
                event_tx.clone(),
                should_stop.clone(),
            );

            let send_task = spawn_openai_send_task(
                ws_sink,
                audio_buffer_rx,
                connection_lost_rx,
                should_stop.clone(),
            );

            // Wait for receive task
            let recv_result = recv_task
                .await
                .unwrap_or(openai_connection::OpenAIReceiveResult {
                    connection_ok: false,
                    quota_exceeded: false,
                });

            // Signal send task
            let _ = connection_lost_tx.send(()).await;

            // Get results from send task
            let send_result = send_task
                .await
                .unwrap_or(openai_connection::OpenAISendResult {
                    audio_rx: mpsc::channel::<AudioChunk>(1).1,
                    pending_chunks: Vec::new(),
                    stopped_by_user: true,
                });

            audio_buffer_rx = send_result.audio_rx;
            pending_chunks = send_result.pending_chunks;

            // Check if we should stop
            if should_stop.load(Ordering::SeqCst) || send_result.stopped_by_user {
                info!("OpenAI transcription session ended");
                break;
            }

            if recv_result.quota_exceeded {
                info!("OpenAI quota exceeded - stopping transcription");
                break;
            }

            if recv_result.connection_ok {
                info!("OpenAI connection closed normally");
                break;
            }

            warn!("OpenAI connection lost, will attempt to reconnect...");
        }

        let _ = audio_forwarder.await;
        Ok(())
    }

    /// Stop the transcription session
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.should_stop.store(true, Ordering::SeqCst);
    }

    /// Clear the session data
    #[allow(dead_code)]
    pub fn clear_session(&self) {
        if let Ok(mut sess) = self.session.lock() {
            *sess = TranscriptionSession::default();
        }
    }
}
