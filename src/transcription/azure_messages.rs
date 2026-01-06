//! Azure OpenAI Realtime API message types for STT
//!
//! Defines the message format for direct Azure OpenAI Realtime WebSocket communication.

use serde::{Deserialize, Serialize};

/// Azure API version for Realtime endpoint
pub const AZURE_API_VERSION: &str = "2024-10-01-preview";

/// Messages sent to Azure OpenAI Realtime API
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum AzureClientMessage {
    /// Session configuration sent after connection
    #[serde(rename = "session.update")]
    SessionUpdate { session: AzureSessionConfig },
    /// Append audio data to the input buffer
    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend { audio: String },
    /// Commit the audio buffer for processing
    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit,
    /// Request a response (triggers transcription)
    #[serde(rename = "response.create")]
    ResponseCreate,
}

/// Session configuration for Azure Realtime API
#[derive(Debug, Serialize)]
pub(crate) struct AzureSessionConfig {
    /// Modalities to use (["text"] for transcription only)
    pub modalities: Vec<String>,
    /// Input audio format (pcm16)
    pub input_audio_format: String,
    /// Transcription configuration
    pub input_audio_transcription: AzureTranscriptionConfig,
}

/// Transcription configuration
#[derive(Debug, Serialize)]
pub(crate) struct AzureTranscriptionConfig {
    /// Model to use (e.g., "gpt-4o-transcribe")
    pub model: String,
    /// Optional language hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

impl AzureSessionConfig {
    /// Create a new session config for STT
    pub fn new(model: &str, language: Option<&str>) -> Self {
        Self {
            modalities: vec!["text".to_string()],
            input_audio_format: "pcm16".to_string(),
            input_audio_transcription: AzureTranscriptionConfig {
                model: model.to_string(),
                language: language.map(String::from),
            },
        }
    }
}

/// Azure Realtime API response messages
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum AzureServerMessage {
    /// Session created
    #[serde(rename = "session.created")]
    SessionCreated {
        #[allow(dead_code)]
        session: Option<AzureSessionInfo>,
    },
    /// Session updated
    #[serde(rename = "session.updated")]
    SessionUpdated {
        #[allow(dead_code)]
        session: Option<AzureSessionInfo>,
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
    /// Response started
    #[serde(rename = "response.created")]
    ResponseCreated,
    /// Response done
    #[serde(rename = "response.done")]
    ResponseDone { response: Option<AzureResponse> },
    /// Error message
    #[serde(rename = "error")]
    Error { error: Option<AzureError> },
    /// Catch-all for other message types
    #[serde(other)]
    Other,
}

/// Azure session information
#[derive(Debug, Deserialize)]
pub(crate) struct AzureSessionInfo {
    #[allow(dead_code)]
    pub id: Option<String>,
    #[allow(dead_code)]
    pub model: Option<String>,
}

/// Azure response object
#[derive(Debug, Deserialize)]
pub(crate) struct AzureResponse {
    #[allow(dead_code)]
    pub id: Option<String>,
    pub output: Option<Vec<AzureOutputItem>>,
}

/// Azure output item
#[derive(Debug, Deserialize)]
pub(crate) struct AzureOutputItem {
    #[allow(dead_code)]
    pub id: Option<String>,
    pub content: Option<Vec<AzureContentItem>>,
}

/// Azure content item
#[derive(Debug, Deserialize)]
pub(crate) struct AzureContentItem {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub content_type: Option<String>,
    pub transcript: Option<String>,
}

/// Azure error details
#[derive(Debug, Deserialize)]
pub(crate) struct AzureError {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub error_type: Option<String>,
    pub message: Option<String>,
}

impl AzureServerMessage {
    /// Convert Azure message to a transcript text if applicable
    pub fn to_transcript_text(&self) -> Option<(bool, String)> {
        match self {
            AzureServerMessage::TranscriptionDelta { delta } => delta
                .as_ref()
                .filter(|s| !s.is_empty())
                .map(|s| (false, s.clone())),
            AzureServerMessage::TranscriptionCompleted { transcript } => transcript
                .as_ref()
                .filter(|s| !s.is_empty())
                .map(|s| (true, s.clone())),
            AzureServerMessage::ResponseDone { response } => {
                // Extract transcript from response output
                response.as_ref().and_then(|r| {
                    r.output.as_ref().and_then(|outputs| {
                        outputs.iter().find_map(|item| {
                            item.content.as_ref().and_then(|contents| {
                                contents.iter().find_map(|c| {
                                    c.transcript
                                        .as_ref()
                                        .filter(|s| !s.is_empty())
                                        .map(|s| (true, s.clone()))
                                })
                            })
                        })
                    })
                })
            }
            _ => None,
        }
    }

    /// Check if this is an error message
    pub fn error_message(&self) -> Option<String> {
        match self {
            AzureServerMessage::Error { error } => error.as_ref().and_then(|e| e.message.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_update_serialization() {
        let msg = AzureClientMessage::SessionUpdate {
            session: AzureSessionConfig::new("gpt-4o-transcribe", Some("en")),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("session.update"));
        assert!(json.contains("gpt-4o-transcribe"));
        assert!(json.contains("pcm16"));
    }

    #[test]
    fn test_audio_append_serialization() {
        let msg = AzureClientMessage::InputAudioBufferAppend {
            audio: "base64data".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("input_audio_buffer.append"));
        assert!(json.contains("base64data"));
    }

    #[test]
    fn test_transcription_completed_deserialization() {
        let json = r#"{"type": "conversation.item.input_audio_transcription.completed", "transcript": "Hello world"}"#;
        let msg: AzureServerMessage = serde_json::from_str(json).unwrap();
        match msg {
            AzureServerMessage::TranscriptionCompleted { transcript } => {
                assert_eq!(transcript.unwrap(), "Hello world");
            }
            _ => panic!("Wrong message type"),
        }
    }
}
