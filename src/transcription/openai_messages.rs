//! OpenAI Realtime API message types for transcription
//!
//! Defines the message format for OpenAI Realtime WebSocket communication.
//! Uses the transcription-specific session type with gpt-4o-transcribe model.

use serde::{Deserialize, Serialize};

/// OpenAI Realtime transcription model
pub const OPENAI_TRANSCRIBE_MODEL: &str = "gpt-4o-transcribe";

/// Messages sent to OpenAI Realtime API
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum OpenAIClientMessage {
    /// Session configuration sent after connection (transcription mode)
    #[serde(rename = "transcription_session.update")]
    TranscriptionSessionUpdate { session: OpenAISessionConfig },
    /// Append audio data to the input buffer
    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend { audio: String },
    /// Commit the audio buffer for processing
    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit,
}

/// Session configuration for OpenAI Realtime transcription API
#[derive(Debug, Serialize)]
pub(crate) struct OpenAISessionConfig {
    /// Input audio format (pcm16)
    pub input_audio_format: String,
    /// Transcription configuration
    pub input_audio_transcription: OpenAITranscriptionConfig,
    /// Noise reduction configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio_noise_reduction: Option<OpenAINoiseReduction>,
    /// Turn detection configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_detection: Option<OpenAITurnDetection>,
}

/// Noise reduction configuration
#[derive(Debug, Serialize)]
pub(crate) struct OpenAINoiseReduction {
    /// Noise reduction type: "near_field" or "far_field"
    #[serde(rename = "type")]
    pub noise_type: String,
}

/// Transcription configuration
#[derive(Debug, Serialize)]
pub(crate) struct OpenAITranscriptionConfig {
    /// Model to use (e.g., "gpt-4o-transcribe")
    pub model: String,
    /// Optional language hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// Turn detection configuration
#[derive(Debug, Serialize)]
pub(crate) struct OpenAITurnDetection {
    /// Detection type: "server_vad" or "semantic_vad"
    #[serde(rename = "type")]
    pub detection_type: String,
    /// Audio volume threshold for speech detection (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f32>,
    /// Audio to include before speech starts (ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix_padding_ms: Option<u32>,
    /// Silence duration to mark end of speech (ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silence_duration_ms: Option<u32>,
}

impl OpenAISessionConfig {
    /// Create a new session config for transcription
    pub fn new(model: &str, language: Option<&str>) -> Self {
        Self {
            input_audio_format: "pcm16".to_string(),
            input_audio_transcription: OpenAITranscriptionConfig {
                model: model.to_string(),
                language: language.map(String::from),
            },
            input_audio_noise_reduction: Some(OpenAINoiseReduction {
                noise_type: "near_field".to_string(),
            }),
            turn_detection: Some(OpenAITurnDetection {
                detection_type: "server_vad".to_string(),
                threshold: Some(0.5),
                prefix_padding_ms: Some(300),
                silence_duration_ms: Some(200),
            }),
        }
    }
}

/// OpenAI Realtime API response messages
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum OpenAIServerMessage {
    /// Session created
    #[serde(rename = "session.created")]
    SessionCreated {
        #[allow(dead_code)]
        session: Option<OpenAISessionInfo>,
    },
    /// Session updated
    #[serde(rename = "session.updated")]
    SessionUpdated {
        #[allow(dead_code)]
        session: Option<OpenAISessionInfo>,
    },
    /// Transcription session updated (for transcription mode)
    #[serde(rename = "transcription_session.created")]
    TranscriptionSessionCreated {
        #[allow(dead_code)]
        session: Option<OpenAISessionInfo>,
    },
    /// Transcription session updated
    #[serde(rename = "transcription_session.updated")]
    TranscriptionSessionUpdated {
        #[allow(dead_code)]
        session: Option<OpenAISessionInfo>,
    },
    /// Partial transcription delta
    #[serde(rename = "conversation.item.input_audio_transcription.delta")]
    TranscriptionDelta { delta: Option<String> },
    /// Completed transcription
    #[serde(rename = "conversation.item.input_audio_transcription.completed")]
    TranscriptionCompleted { transcript: Option<String> },
    /// Input audio buffer committed
    #[serde(rename = "input_audio_buffer.committed")]
    InputAudioBufferCommitted,
    /// Input audio buffer cleared
    #[serde(rename = "input_audio_buffer.cleared")]
    InputAudioBufferCleared,
    /// Input audio buffer speech started (VAD detected speech)
    #[serde(rename = "input_audio_buffer.speech_started")]
    InputAudioBufferSpeechStarted,
    /// Input audio buffer speech stopped (VAD detected silence)
    #[serde(rename = "input_audio_buffer.speech_stopped")]
    InputAudioBufferSpeechStopped,
    /// Error message
    #[serde(rename = "error")]
    Error { error: Option<OpenAIError> },
    /// Catch-all for other message types
    #[serde(other)]
    Other,
}

/// OpenAI session information
#[derive(Debug, Deserialize)]
pub(crate) struct OpenAISessionInfo {
    #[allow(dead_code)]
    pub id: Option<String>,
    #[allow(dead_code)]
    pub model: Option<String>,
}

/// OpenAI error details
#[derive(Debug, Deserialize)]
pub(crate) struct OpenAIError {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub error_type: Option<String>,
    pub message: Option<String>,
}

impl OpenAIServerMessage {
    /// Convert OpenAI message to a transcript text if applicable
    /// Returns (is_committed, text) where is_committed is true for final segments
    pub fn to_transcript_text(&self) -> Option<(bool, String)> {
        match self {
            OpenAIServerMessage::TranscriptionDelta { delta } => delta
                .as_ref()
                .filter(|s| !s.is_empty())
                .map(|s| (false, s.clone())),
            OpenAIServerMessage::TranscriptionCompleted { transcript } => transcript
                .as_ref()
                .filter(|s| !s.is_empty())
                .map(|s| (true, s.clone())),
            _ => None,
        }
    }

    /// Check if this is an error message
    pub fn error_message(&self) -> Option<String> {
        match self {
            OpenAIServerMessage::Error { error } => error.as_ref().and_then(|e| e.message.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcription_session_update_serialization() {
        let msg = OpenAIClientMessage::TranscriptionSessionUpdate {
            session: OpenAISessionConfig::new("gpt-4o-transcribe", Some("en")),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("transcription_session.update"));
        assert!(json.contains("gpt-4o-transcribe"));
        assert!(json.contains("pcm16"));
    }

    #[test]
    fn test_audio_append_serialization() {
        let msg = OpenAIClientMessage::InputAudioBufferAppend {
            audio: "base64data".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("input_audio_buffer.append"));
        assert!(json.contains("base64data"));
    }

    #[test]
    fn test_transcription_completed_deserialization() {
        let json = r#"{"type": "conversation.item.input_audio_transcription.completed", "transcript": "Hello world"}"#;
        let msg: OpenAIServerMessage = serde_json::from_str(json).unwrap();
        match msg {
            OpenAIServerMessage::TranscriptionCompleted { transcript } => {
                assert_eq!(transcript.unwrap(), "Hello world");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_transcription_delta_deserialization() {
        let json =
            r#"{"type": "conversation.item.input_audio_transcription.delta", "delta": "Hello"}"#;
        let msg: OpenAIServerMessage = serde_json::from_str(json).unwrap();
        match msg {
            OpenAIServerMessage::TranscriptionDelta { delta } => {
                assert_eq!(delta.unwrap(), "Hello");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_to_transcript_text() {
        let delta = OpenAIServerMessage::TranscriptionDelta {
            delta: Some("Hello".to_string()),
        };
        let (is_committed, text) = delta.to_transcript_text().unwrap();
        assert!(!is_committed);
        assert_eq!(text, "Hello");

        let completed = OpenAIServerMessage::TranscriptionCompleted {
            transcript: Some("Hello world".to_string()),
        };
        let (is_committed, text) = completed.to_transcript_text().unwrap();
        assert!(is_committed);
        assert_eq!(text, "Hello world");
    }
}
