//! Azure OpenAI Realtime API WebSocket connection handling
//!
//! Manages the WebSocket connection to Azure OpenAI for direct STT using GPT-4o Transcribe.
//! Uses a different protocol than the ElevenLabs/VIPS proxy connections.

use super::azure_messages::{
    AzureClientMessage, AzureServerMessage, AzureSessionConfig, AZURE_API_VERSION,
};
use super::session::TranscriptionSession;
use super::TranscriptEvent;
use crate::audio::AudioChunk;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio::time::interval;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, trace, warn};

/// Ping interval in seconds to keep WebSocket connections alive
const PING_INTERVAL_SECS: u64 = 30;

/// Result of Azure receive task
pub(crate) struct AzureReceiveResult {
    pub(crate) connection_ok: bool,
    pub(crate) quota_exceeded: bool,
}

/// Result of Azure send task
pub(crate) struct AzureSendResult {
    pub(crate) audio_rx: mpsc::Receiver<AudioChunk>,
    pub(crate) pending_chunks: Vec<AudioChunk>,
    pub(crate) stopped_by_user: bool,
}

/// Build Azure WebSocket URL
pub(crate) fn build_azure_ws_url(endpoint_url: &str, stt_deployment: &str) -> String {
    // Remove trailing slash if present
    let endpoint = endpoint_url.trim_end_matches('/');

    // Convert https:// to wss://
    let ws_endpoint = endpoint
        .replace("https://", "wss://")
        .replace("http://", "ws://");

    format!(
        "{}/openai/realtime?api-version={}&deployment={}",
        ws_endpoint, AZURE_API_VERSION, stt_deployment
    )
}

/// Build Azure WebSocket request with api-key authentication
pub(crate) fn build_azure_ws_request(
    ws_url: &str,
    host: &str,
    api_key: &str,
) -> Result<http::Request<()>, String> {
    http::Request::builder()
        .uri(ws_url)
        .header("Host", host)
        .header("api-key", api_key)
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Key", super::helpers::generate_ws_key())
        .header("Sec-WebSocket-Version", "13")
        .body(())
        .map_err(|e| e.to_string())
}

/// Send Azure session initialization message
pub(crate) async fn send_session_init<S>(
    ws_sink: &mut S,
    model: &str,
    language: Option<&str>,
) -> Result<(), String>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let session_config = AzureSessionConfig::new(model, language);
    let msg = AzureClientMessage::SessionUpdate {
        session: session_config,
    };

    let json = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
    info!("Sending Azure session.update: {}", json);

    ws_sink
        .send(Message::Text(json))
        .await
        .map_err(|e| e.to_string())
}

/// Spawn the Azure receive task that handles incoming WebSocket messages
pub(crate) fn spawn_azure_receive_task(
    mut ws_stream: impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + Unpin
        + Send
        + 'static,
    session: Arc<Mutex<TranscriptionSession>>,
    event_tx: broadcast::Sender<TranscriptEvent>,
    should_stop: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<AzureReceiveResult> {
    tokio::spawn(async move {
        let mut connection_ok = true;
        let quota_exceeded = false;

        while let Some(msg_result) = ws_stream.next().await {
            if should_stop.load(Ordering::SeqCst) {
                break;
            }

            match msg_result {
                Ok(Message::Text(text)) => {
                    trace!("Azure message: {}", text);
                    match serde_json::from_str::<AzureServerMessage>(&text) {
                        Ok(azure_msg) => {
                            // Check for errors
                            if let Some(error_msg) = azure_msg.error_message() {
                                // The "buffer too small" with 0.00ms is expected when stopping
                                // recording - Azure's server VAD already committed the audio
                                if error_msg.contains("buffer too small")
                                    && error_msg.contains("0.00ms")
                                {
                                    debug!("Azure buffer empty on stop (expected): {}", error_msg);
                                    continue;
                                }
                                error!("Azure STT error: {}", error_msg);
                                let _ =
                                    event_tx.send(TranscriptEvent::Error { message: error_msg });
                                continue;
                            }

                            // Convert Azure message to transcript event
                            if let Some((is_final, text)) = azure_msg.to_transcript_text() {
                                update_azure_session_state(&session, is_final, &text);

                                let event = if is_final {
                                    debug!("Azure committed transcript: {}", text);
                                    TranscriptEvent::CommittedTranscript { text }
                                } else {
                                    trace!("Azure partial transcript: {}", text);
                                    TranscriptEvent::PartialTranscript { text }
                                };
                                let _ = event_tx.send(event);
                            }

                            // Log session events
                            match &azure_msg {
                                AzureServerMessage::SessionCreated { .. } => {
                                    info!("Azure session created");
                                }
                                AzureServerMessage::SessionUpdated { .. } => {
                                    info!("Azure session updated");
                                }
                                AzureServerMessage::InputAudioBufferCommitted => {
                                    debug!("Azure audio buffer committed");
                                }
                                AzureServerMessage::ResponseCreated => {
                                    debug!("Azure response created");
                                }
                                AzureServerMessage::ResponseDone { .. } => {
                                    debug!("Azure response done");
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse Azure message: {} - {}", e, text);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Azure WebSocket closed by server");
                    connection_ok = false;
                    preserve_azure_partial(&session, "connection close");
                    if !quota_exceeded {
                        let _ = event_tx.send(TranscriptEvent::ConnectionLost);
                    }
                    break;
                }
                Ok(Message::Ping(_)) => {
                    trace!("Received Azure WebSocket ping");
                }
                Ok(Message::Pong(_)) => {
                    trace!("Received Azure WebSocket pong");
                }
                Err(e) => {
                    error!("Azure WebSocket receive error: {}", e);
                    connection_ok = false;
                    preserve_azure_partial(&session, "receive error");
                    let _ = event_tx.send(TranscriptEvent::ConnectionLost);
                    break;
                }
                _ => {}
            }
        }

        AzureReceiveResult {
            connection_ok,
            quota_exceeded,
        }
    })
}

/// Spawn the Azure send task that forwards audio chunks
pub(crate) fn spawn_azure_send_task<S>(
    mut ws_sink: S,
    mut audio_rx: mpsc::Receiver<AudioChunk>,
    mut connection_lost_rx: mpsc::Receiver<()>,
    should_stop: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<AzureSendResult>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        info!("Azure send task started");
        let base64_engine = base64::engine::general_purpose::STANDARD;
        let mut pending_chunks: Vec<AudioChunk> = Vec::new();
        let mut sent_buffer: VecDeque<AudioChunk> = VecDeque::new();
        let max_buffer_secs = 30.0;
        let mut chunks_sent = 0u64;

        let mut ping_interval = interval(Duration::from_secs(PING_INTERVAL_SECS));
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        #[allow(unused_assignments)]
        let mut connection_lost = false;

        loop {
            tokio::select! {
                biased;

                _ = connection_lost_rx.recv() => {
                    connection_lost = true;
                    info!("Azure send task received connection lost signal");
                    break;
                }
                _ = ping_interval.tick() => {
                    if ws_sink.send(Message::Ping(vec![])).await.is_err() {
                        warn!("Failed to send Azure keepalive ping");
                        connection_lost = true;
                        break;
                    }
                    trace!("Sent Azure keepalive ping");
                }
                chunk = audio_rx.recv() => {
                    if should_stop.load(Ordering::SeqCst) {
                        info!("Azure send task: should_stop flag set, sending commit");
                        // Send commit + response.create before closing
                        if let Err(e) = send_azure_commit_and_create(&mut ws_sink).await {
                            warn!("Failed to send Azure commit: {}", e);
                        }
                        let _ = ws_sink.close().await;
                        return AzureSendResult {
                            audio_rx,
                            pending_chunks: Vec::new(),
                            stopped_by_user: true,
                        };
                    }
                    match chunk {
                        Some(audio_chunk) => {
                            chunks_sent += 1;
                            let duration_ms = (audio_chunk.samples.len() as f64 / audio_chunk.sample_rate as f64) * 1000.0;
                            // Check if audio has actual content (not silence)
                            let max_sample = audio_chunk.samples.iter().map(|s| s.abs()).max().unwrap_or(0);
                            if chunks_sent == 1 || chunks_sent.is_multiple_of(50) {
                                info!(
                                    "Azure send task: sending chunk #{}, {} samples, {:.1}ms, max_amplitude={}",
                                    chunks_sent,
                                    audio_chunk.samples.len(),
                                    duration_ms,
                                    max_sample
                                );
                            }
                            match send_azure_audio_chunk(&mut ws_sink, &audio_chunk, &base64_engine).await {
                                Ok(()) => {
                                    sent_buffer.push_back(audio_chunk);
                                    trim_azure_sent_buffer(&mut sent_buffer, max_buffer_secs);
                                }
                                Err(_) => {
                                    error!("Failed to send Azure audio chunk");
                                    pending_chunks.push(audio_chunk);
                                    connection_lost = true;
                                    break;
                                }
                            }
                        }
                        None => {
                            info!("Azure audio buffer channel closed after sending {} chunks", chunks_sent);
                            if let Err(e) = send_azure_commit_and_create(&mut ws_sink).await {
                                warn!("Failed to send Azure commit: {}", e);
                            }
                            let _ = ws_sink.close().await;
                            return AzureSendResult {
                                audio_rx,
                                pending_chunks: Vec::new(),
                                stopped_by_user: true,
                            };
                        }
                    }
                }
            }
        }

        if connection_lost {
            pending_chunks =
                recover_azure_buffered_chunks(sent_buffer, pending_chunks, &mut audio_rx);
        }

        info!(
            "Azure send task exiting after sending {} chunks",
            chunks_sent
        );
        AzureSendResult {
            audio_rx,
            pending_chunks,
            stopped_by_user: false,
        }
    })
}

/// Send audio chunk to Azure in the Realtime API format
async fn send_azure_audio_chunk<S>(
    ws_sink: &mut S,
    chunk: &AudioChunk,
    base64_engine: &base64::engine::GeneralPurpose,
) -> Result<(), ()>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    // Convert samples to bytes (PCM16 little-endian)
    let bytes: Vec<u8> = chunk
        .samples
        .iter()
        .flat_map(|&s| s.to_le_bytes())
        .collect();

    let audio_base64 = base64_engine.encode(&bytes);
    let msg = AzureClientMessage::InputAudioBufferAppend {
        audio: audio_base64,
    };

    if let Ok(json) = serde_json::to_string(&msg) {
        ws_sink.send(Message::Text(json)).await.map_err(|_| ())?;
    }
    Ok(())
}

/// Send commit and response.create to finalize transcription
async fn send_azure_commit_and_create<S>(ws_sink: &mut S) -> Result<(), String>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    // Send input_audio_buffer.commit
    let commit_msg = serde_json::to_string(&AzureClientMessage::InputAudioBufferCommit)
        .map_err(|e| e.to_string())?;
    ws_sink
        .send(Message::Text(commit_msg))
        .await
        .map_err(|e| e.to_string())?;

    // Send response.create to trigger transcription
    let create_msg =
        serde_json::to_string(&AzureClientMessage::ResponseCreate).map_err(|e| e.to_string())?;
    ws_sink
        .send(Message::Text(create_msg))
        .await
        .map_err(|e| e.to_string())?;

    debug!("Sent Azure commit and response.create");
    Ok(())
}

/// Trim the sent buffer to stay within max duration
fn trim_azure_sent_buffer(sent_buffer: &mut VecDeque<AudioChunk>, max_buffer_secs: f64) {
    let mut current_duration = 0.0;
    for c in sent_buffer.iter() {
        current_duration += c.samples.len() as f64 / c.sample_rate as f64;
    }

    while current_duration > max_buffer_secs && sent_buffer.len() > 1 {
        if let Some(removed) = sent_buffer.pop_front() {
            current_duration -= removed.samples.len() as f64 / removed.sample_rate as f64;
        }
    }
}

/// Recover buffered chunks when connection is lost
fn recover_azure_buffered_chunks(
    sent_buffer: VecDeque<AudioChunk>,
    pending_chunks: Vec<AudioChunk>,
    audio_rx: &mut mpsc::Receiver<AudioChunk>,
) -> Vec<AudioChunk> {
    info!(
        "Recovering {} Azure sent chunks ({:.1}s)",
        sent_buffer.len(),
        sent_buffer
            .iter()
            .map(|c| c.samples.len() as f64 / c.sample_rate as f64)
            .sum::<f64>()
    );

    let mut all_pending = Vec::from(sent_buffer);
    all_pending.extend(pending_chunks);

    while let Ok(chunk) = audio_rx.try_recv() {
        all_pending.push(chunk);
    }

    info!(
        "Buffered {} Azure audio chunks for resend",
        all_pending.len()
    );

    all_pending
}

/// Update session state based on Azure transcript
fn update_azure_session_state(
    session: &Arc<Mutex<TranscriptionSession>>,
    is_final: bool,
    text: &str,
) {
    if let Ok(mut sess) = session.lock() {
        if is_final {
            sess.committed_segments.push(text.to_string());
            sess.partial_transcript = None;
        } else {
            sess.partial_transcript = Some(text.to_string());
        }
    }
}

/// Preserve any partial transcript as committed
fn preserve_azure_partial(session: &Arc<Mutex<TranscriptionSession>>, reason: &str) {
    if let Ok(mut sess) = session.lock() {
        if let Some(partial) = sess.partial_transcript.take() {
            if !partial.trim().is_empty() {
                info!(
                    "Preserving Azure partial transcript before {}: {} chars",
                    reason,
                    partial.len()
                );
                sess.committed_segments.push(partial);
            }
        }
    }
}

/// Resend buffered Azure audio chunks after reconnection
pub(crate) async fn resend_azure_buffered_chunks<S>(
    ws_sink: &mut S,
    pending_chunks: &mut Vec<AudioChunk>,
) -> Result<(), ()>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    if pending_chunks.is_empty() {
        return Ok(());
    }

    info!(
        "Resending {} Azure buffered audio chunks",
        pending_chunks.len()
    );
    let base64_engine = base64::engine::general_purpose::STANDARD;

    for chunk in pending_chunks.drain(..) {
        let bytes: Vec<u8> = chunk
            .samples
            .iter()
            .flat_map(|&s| s.to_le_bytes())
            .collect();

        let audio_base64 = base64_engine.encode(&bytes);
        let msg = AzureClientMessage::InputAudioBufferAppend {
            audio: audio_base64,
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            if ws_sink.send(Message::Text(json)).await.is_err() {
                error!("Failed to resend Azure buffered audio chunk");
                return Err(());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_azure_ws_url() {
        let url = build_azure_ws_url("https://myresource.openai.azure.com", "gpt-4o-transcribe");
        assert!(url.starts_with("wss://"));
        assert!(url.contains("api-version="));
        assert!(url.contains("deployment=gpt-4o-transcribe"));
    }

    #[test]
    fn test_build_azure_ws_url_trailing_slash() {
        let url = build_azure_ws_url("https://myresource.openai.azure.com/", "gpt-4o-transcribe");
        assert!(!url.contains("//openai"));
    }
}
