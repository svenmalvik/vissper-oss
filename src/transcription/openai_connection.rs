//! OpenAI Realtime API WebSocket connection handling
//!
//! Manages the WebSocket connection to OpenAI for direct STT using GPT-4o Transcribe.
//! Uses the transcription-specific endpoint with intent=transcription parameter.

use super::openai_messages::{
    OpenAIClientMessage, OpenAIServerMessage, OpenAISessionConfig, OPENAI_TRANSCRIBE_MODEL,
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

/// OpenAI Realtime API WebSocket URL for transcription
const OPENAI_REALTIME_URL: &str = "wss://api.openai.com/v1/realtime?intent=transcription";

/// Result of OpenAI receive task
pub(crate) struct OpenAIReceiveResult {
    pub(crate) connection_ok: bool,
    pub(crate) quota_exceeded: bool,
}

/// Result of OpenAI send task
pub(crate) struct OpenAISendResult {
    pub(crate) audio_rx: mpsc::Receiver<AudioChunk>,
    pub(crate) pending_chunks: Vec<AudioChunk>,
    pub(crate) stopped_by_user: bool,
}

/// Build OpenAI WebSocket URL
pub(crate) fn build_openai_ws_url() -> String {
    OPENAI_REALTIME_URL.to_string()
}

/// Build OpenAI WebSocket request with Bearer token authentication
pub(crate) fn build_openai_ws_request(
    ws_url: &str,
    api_key: &str,
) -> Result<http::Request<()>, String> {
    http::Request::builder()
        .uri(ws_url)
        .header("Host", "api.openai.com")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("OpenAI-Beta", "realtime=v1")
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Key", super::helpers::generate_ws_key())
        .header("Sec-WebSocket-Version", "13")
        .body(())
        .map_err(|e| e.to_string())
}

/// Send OpenAI session initialization message (transcription mode)
pub(crate) async fn send_session_init<S>(
    ws_sink: &mut S,
    language: Option<&str>,
) -> Result<(), String>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let session_config = OpenAISessionConfig::new(OPENAI_TRANSCRIBE_MODEL, language);
    let msg = OpenAIClientMessage::TranscriptionSessionUpdate {
        session: session_config,
    };

    let json = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
    info!("Sending OpenAI transcription_session.update: {}", json);

    ws_sink
        .send(Message::Text(json))
        .await
        .map_err(|e| e.to_string())
}

/// Spawn the OpenAI receive task that handles incoming WebSocket messages
pub(crate) fn spawn_openai_receive_task(
    mut ws_stream: impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + Unpin
        + Send
        + 'static,
    session: Arc<Mutex<TranscriptionSession>>,
    event_tx: broadcast::Sender<TranscriptEvent>,
    should_stop: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<OpenAIReceiveResult> {
    tokio::spawn(async move {
        let mut connection_ok = true;
        let quota_exceeded = false;

        while let Some(msg_result) = ws_stream.next().await {
            if should_stop.load(Ordering::SeqCst) {
                break;
            }

            match msg_result {
                Ok(Message::Text(text)) => {
                    trace!("OpenAI message: {}", text);
                    match serde_json::from_str::<OpenAIServerMessage>(&text) {
                        Ok(openai_msg) => {
                            // Check for errors
                            if let Some(error_msg) = openai_msg.error_message() {
                                // The "buffer too small" error is expected when stopping
                                if error_msg.contains("buffer too small")
                                    || error_msg.contains("empty")
                                {
                                    debug!("OpenAI buffer empty on stop (expected): {}", error_msg);
                                    continue;
                                }
                                error!("OpenAI STT error: {}", error_msg);
                                let _ =
                                    event_tx.send(TranscriptEvent::Error { message: error_msg });
                                continue;
                            }

                            // Convert OpenAI message to transcript event
                            if let Some((is_final, text)) = openai_msg.to_transcript_text() {
                                update_openai_session_state(&session, is_final, &text);

                                let event = if is_final {
                                    debug!("OpenAI committed transcript: {}", text);
                                    TranscriptEvent::CommittedTranscript { text }
                                } else {
                                    trace!("OpenAI partial transcript: {}", text);
                                    TranscriptEvent::PartialTranscript { text }
                                };
                                let _ = event_tx.send(event);
                            }

                            // Log session events
                            match &openai_msg {
                                OpenAIServerMessage::SessionCreated { .. } => {
                                    info!("OpenAI session created");
                                }
                                OpenAIServerMessage::SessionUpdated { .. } => {
                                    info!("OpenAI session updated");
                                }
                                OpenAIServerMessage::TranscriptionSessionCreated { .. } => {
                                    info!("OpenAI transcription session created");
                                }
                                OpenAIServerMessage::TranscriptionSessionUpdated { .. } => {
                                    info!("OpenAI transcription session updated");
                                }
                                OpenAIServerMessage::InputAudioBufferCommitted => {
                                    debug!("OpenAI audio buffer committed");
                                }
                                OpenAIServerMessage::InputAudioBufferSpeechStarted => {
                                    debug!("OpenAI VAD: speech started");
                                }
                                OpenAIServerMessage::InputAudioBufferSpeechStopped => {
                                    debug!("OpenAI VAD: speech stopped");
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse OpenAI message: {} - {}", e, text);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("OpenAI WebSocket closed by server");
                    connection_ok = false;
                    preserve_openai_partial(&session, "connection close");
                    if !quota_exceeded {
                        let _ = event_tx.send(TranscriptEvent::ConnectionLost);
                    }
                    break;
                }
                Ok(Message::Ping(_)) => {
                    trace!("Received OpenAI WebSocket ping");
                }
                Ok(Message::Pong(_)) => {
                    trace!("Received OpenAI WebSocket pong");
                }
                Err(e) => {
                    error!("OpenAI WebSocket receive error: {}", e);
                    connection_ok = false;
                    preserve_openai_partial(&session, "receive error");
                    let _ = event_tx.send(TranscriptEvent::ConnectionLost);
                    break;
                }
                _ => {}
            }
        }

        OpenAIReceiveResult {
            connection_ok,
            quota_exceeded,
        }
    })
}

/// Spawn the OpenAI send task that forwards audio chunks
pub(crate) fn spawn_openai_send_task<S>(
    mut ws_sink: S,
    mut audio_rx: mpsc::Receiver<AudioChunk>,
    mut connection_lost_rx: mpsc::Receiver<()>,
    should_stop: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<OpenAISendResult>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        info!("OpenAI send task started");
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
                    info!("OpenAI send task received connection lost signal");
                    break;
                }
                _ = ping_interval.tick() => {
                    if ws_sink.send(Message::Ping(vec![])).await.is_err() {
                        warn!("Failed to send OpenAI keepalive ping");
                        connection_lost = true;
                        break;
                    }
                    trace!("Sent OpenAI keepalive ping");
                }
                chunk = audio_rx.recv() => {
                    if should_stop.load(Ordering::SeqCst) {
                        info!("OpenAI send task: should_stop flag set, sending commit");
                        // Send commit before closing
                        if let Err(e) = send_openai_commit(&mut ws_sink).await {
                            warn!("Failed to send OpenAI commit: {}", e);
                        }
                        let _ = ws_sink.close().await;
                        return OpenAISendResult {
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
                                    "OpenAI send task: sending chunk #{}, {} samples, {:.1}ms, max_amplitude={}",
                                    chunks_sent,
                                    audio_chunk.samples.len(),
                                    duration_ms,
                                    max_sample
                                );
                            }
                            match send_openai_audio_chunk(&mut ws_sink, &audio_chunk, &base64_engine).await {
                                Ok(()) => {
                                    sent_buffer.push_back(audio_chunk);
                                    trim_openai_sent_buffer(&mut sent_buffer, max_buffer_secs);
                                }
                                Err(_) => {
                                    error!("Failed to send OpenAI audio chunk");
                                    pending_chunks.push(audio_chunk);
                                    connection_lost = true;
                                    break;
                                }
                            }
                        }
                        None => {
                            info!("OpenAI audio buffer channel closed after sending {} chunks", chunks_sent);
                            if let Err(e) = send_openai_commit(&mut ws_sink).await {
                                warn!("Failed to send OpenAI commit: {}", e);
                            }
                            let _ = ws_sink.close().await;
                            return OpenAISendResult {
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
                recover_openai_buffered_chunks(sent_buffer, pending_chunks, &mut audio_rx);
        }

        info!(
            "OpenAI send task exiting after sending {} chunks",
            chunks_sent
        );
        OpenAISendResult {
            audio_rx,
            pending_chunks,
            stopped_by_user: false,
        }
    })
}

/// Send audio chunk to OpenAI in the Realtime API format
async fn send_openai_audio_chunk<S>(
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
    let msg = OpenAIClientMessage::InputAudioBufferAppend {
        audio: audio_base64,
    };

    if let Ok(json) = serde_json::to_string(&msg) {
        ws_sink.send(Message::Text(json)).await.map_err(|_| ())?;
    }
    Ok(())
}

/// Send commit to finalize transcription
async fn send_openai_commit<S>(ws_sink: &mut S) -> Result<(), String>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    // Send input_audio_buffer.commit
    let commit_msg = serde_json::to_string(&OpenAIClientMessage::InputAudioBufferCommit)
        .map_err(|e| e.to_string())?;
    ws_sink
        .send(Message::Text(commit_msg))
        .await
        .map_err(|e| e.to_string())?;

    debug!("Sent OpenAI commit");
    Ok(())
}

/// Trim the sent buffer to stay within max duration
fn trim_openai_sent_buffer(sent_buffer: &mut VecDeque<AudioChunk>, max_buffer_secs: f64) {
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
fn recover_openai_buffered_chunks(
    sent_buffer: VecDeque<AudioChunk>,
    pending_chunks: Vec<AudioChunk>,
    audio_rx: &mut mpsc::Receiver<AudioChunk>,
) -> Vec<AudioChunk> {
    info!(
        "Recovering {} OpenAI sent chunks ({:.1}s)",
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
        "Buffered {} OpenAI audio chunks for resend",
        all_pending.len()
    );

    all_pending
}

/// Update session state based on OpenAI transcript
fn update_openai_session_state(
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
fn preserve_openai_partial(session: &Arc<Mutex<TranscriptionSession>>, reason: &str) {
    if let Ok(mut sess) = session.lock() {
        if let Some(partial) = sess.partial_transcript.take() {
            if !partial.trim().is_empty() {
                info!(
                    "Preserving OpenAI partial transcript before {}: {} chars",
                    reason,
                    partial.len()
                );
                sess.committed_segments.push(partial);
            }
        }
    }
}

/// Resend buffered OpenAI audio chunks after reconnection
pub(crate) async fn resend_openai_buffered_chunks<S>(
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
        "Resending {} OpenAI buffered audio chunks",
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
        let msg = OpenAIClientMessage::InputAudioBufferAppend {
            audio: audio_base64,
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            if ws_sink.send(Message::Text(json)).await.is_err() {
                error!("Failed to resend OpenAI buffered audio chunk");
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
    fn test_build_openai_ws_url() {
        let url = build_openai_ws_url();
        assert!(url.starts_with("wss://"));
        assert!(url.contains("api.openai.com"));
        assert!(url.contains("intent=transcription"));
    }
}
