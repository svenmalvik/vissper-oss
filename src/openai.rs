//! Direct OpenAI client for transcript polishing.
//!
//! This module provides a client that connects directly to OpenAI's Chat Completions API.
//! Users provide their own OpenAI API key.

use crate::error::ResponseError;
use crate::keychain::OpenAICredentials;
use crate::response::{language_code_to_name, PolishConfig};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, instrument, warn};
use zeroize::Zeroize;

/// Maximum number of retry attempts for transient failures.
const MAX_RETRIES: u32 = 3;

/// Initial delay between retries (doubles with each attempt).
const INITIAL_RETRY_DELAY_MS: u64 = 1000;

/// OpenAI API endpoint
const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// Default model for polishing
const POLISH_MODEL: &str = "gpt-5.2";

/// Client for direct OpenAI Chat Completions API calls.
pub(crate) struct OpenAIClient {
    api_key: String,
    client: reqwest::Client,
}

/// Request body for OpenAI Chat Completions API.
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

/// Message in the OpenAI request.
#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// Response from OpenAI Chat Completions API.
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

/// Choice in the response.
#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

/// Response message content.
#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

/// System prompt template for basic transcript polishing.
/// Use `{language}` placeholder for the target language.
const POLISH_PROMPT_TEMPLATE: &str = r#"You are an expert copy editor. Your task is to polish and copyedit the following transcript for improved readability and grammar while preserving the original meaning and tone. Fix any obvious transcription errors, improve punctuation, and ensure proper sentence structure. Do not add new content or change the meaning. The output MUST be in {language}. Do not translate to any other language.

IMPORTANT: The transcript may contain screenshot references in markdown image format like `![Screenshot](screenshots/filename.png)`. These must be preserved exactly as they appear, in their original positions within the transcript. Do not modify, remove, or relocate these screenshot references.

Return only the polished transcript without any additional commentary."#;

/// System prompt template for live meeting recording.
/// Use `{language}` placeholder for the target language.
const LIVE_MEETING_PROMPT_TEMPLATE: &str = r#"You are an expert meeting assistant. Your task is to analyze the following meeting transcript and generate a comprehensive, well-structured output. The output MUST be in {language}. Do not translate to any other language.

Extract and organize the key information into the following sections:

## Summary
Provide a concise overview of what the meeting was about and its main outcomes. Use as many sentences as needed to capture the essence, proportional to the meeting length and complexity.

## Main Items
List the most important topics or points discussed in the meeting. Include only the most significant items, maximum 7 bullet points.

## Action Items
List all tasks, assignments, or commitments that were made during the meeting. Include who is responsible if mentioned.

## Decisions
List any decisions that were made during the meeting.

## Follow-ups
List any items that need follow-up, further discussion, or were deferred to a future meeting.

---

## Transcript
Condense and polish the transcript for readability. Remove filler words, meaningless acknowledgments (e.g., "Yeah.", "Right.", "Uh-huh."), and back-and-forth exchanges that add no substantive content. Keep the essential points and meaningful dialogue while removing conversational noise. Fix any transcription errors, improve punctuation, and ensure proper sentence structure. Structure the polished transcript into clear paragraphs, grouping related content together. Use line breaks between different topics or speakers for easy reading.

IMPORTANT: The transcript may contain screenshot references in markdown image format like `![Screenshot](screenshots/filename.png)`. These must be preserved exactly as they appear, in their original positions within the transcript. Do not modify, remove, or relocate these screenshot references.

If a section has no relevant content from the transcript, write "None identified" for that section.

Return the output in the format above with the section headers as shown."#;

/// Select the appropriate prompt based on config, with language injected
fn select_prompt(config: &PolishConfig) -> String {
    let language = language_code_to_name(&config.language_code);
    let template = match config.prompt_type.as_deref() {
        Some("live_meeting") => LIVE_MEETING_PROMPT_TEMPLATE,
        _ => POLISH_PROMPT_TEMPLATE,
    };
    template.replace("{language}", language)
}

impl OpenAIClient {
    /// Create a new OpenAI client from credentials.
    pub(crate) fn new(creds: &OpenAICredentials) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client for OpenAIClient")?;

        Ok(Self {
            api_key: creds.api_key.clone(),
            client,
        })
    }

    /// Polish a transcript using OpenAI Chat Completions API.
    ///
    /// Sends the raw transcript to OpenAI for copyediting and polishing.
    /// Includes retry logic for transient network failures.
    #[instrument(skip(self, transcript, config), fields(transcript_len = transcript.len()))]
    pub(crate) async fn polish_transcript(
        &self,
        transcript: &str,
        config: &PolishConfig,
    ) -> Result<String, ResponseError> {
        let prompt = select_prompt(config);
        let request_body = ChatCompletionRequest {
            model: POLISH_MODEL.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: prompt,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: transcript.to_string(),
                },
            ],
        };

        let mut last_error: Option<ResponseError> = None;
        let mut retry_delay = Duration::from_millis(INITIAL_RETRY_DELAY_MS);

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                warn!(
                    attempt = attempt,
                    max_retries = MAX_RETRIES,
                    delay_ms = retry_delay.as_millis(),
                    "Retrying OpenAI polish request after transient failure"
                );
                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2;
            }

            let result = self
                .client
                .post(OPENAI_API_URL)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            match result {
                Ok(response) => {
                    if response.status().is_success() {
                        let chat_response: ChatCompletionResponse =
                            response.json().await.map_err(|e| {
                                ResponseError::InvalidResponse(format!(
                                    "Failed to parse OpenAI response: {}",
                                    e
                                ))
                            })?;

                        // Extract text from response
                        let polished_text = Self::extract_text(&chat_response)?;

                        if attempt > 0 {
                            info!(
                                attempt = attempt,
                                "OpenAI polish request succeeded after retry"
                            );
                        }

                        return Ok(polished_text);
                    }

                    let status = response.status().as_u16();
                    let message = response.text().await.unwrap_or_default();

                    let error = ResponseError::ServerError { status, message };

                    // Retry on 5xx server errors
                    if (500..600).contains(&status) && attempt < MAX_RETRIES {
                        warn!(
                            status = status,
                            attempt = attempt,
                            "Server error, will retry"
                        );
                        last_error = Some(error);
                        continue;
                    }

                    return Err(error);
                }
                Err(e) => {
                    // Retry on network errors
                    if Self::is_retryable_error(&e) && attempt < MAX_RETRIES {
                        warn!(error = %e, attempt = attempt, "Network error, will retry");
                        last_error = Some(ResponseError::Network(e));
                        continue;
                    }

                    return Err(ResponseError::Network(e));
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| ResponseError::InvalidResponse("Unexpected retry loop exit".into())))
    }

    /// Extract text from the OpenAI response structure.
    fn extract_text(response: &ChatCompletionResponse) -> Result<String, ResponseError> {
        response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .filter(|text| !text.is_empty())
            .ok_or_else(|| {
                ResponseError::InvalidResponse("No text content in OpenAI response".into())
            })
    }

    /// Check if a reqwest error is retryable (transient).
    fn is_retryable_error(error: &reqwest::Error) -> bool {
        error.is_timeout() || error.is_connect() || error.is_request()
    }
}

impl Drop for OpenAIClient {
    fn drop(&mut self) {
        // Clear API key from memory
        self.api_key.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_request_serialization() {
        let request = ChatCompletionRequest {
            model: "gpt-5.2".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "System prompt".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hello world".to_string(),
                },
            ],
        };

        let json = serde_json::to_string(&request).expect("Failed to serialize");
        assert!(json.contains("gpt-5.2"));
        assert!(json.contains("system"));
        assert!(json.contains("Hello world"));
    }

    #[test]
    fn test_openai_response_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1677652288,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Polished text here"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 9,
                "completion_tokens": 12,
                "total_tokens": 21
            }
        }"#;

        let response: ChatCompletionResponse =
            serde_json::from_str(json).expect("Failed to deserialize");
        let text = OpenAIClient::extract_text(&response).expect("Failed to extract text");
        assert_eq!(text, "Polished text here");
    }

    #[test]
    fn test_select_prompt_injects_language() {
        let config = PolishConfig {
            reasoning_effort: None,
            prompt_type: None,
            language_code: "en".to_string(),
        };
        let prompt = select_prompt(&config);
        assert!(prompt.contains("The output MUST be in English"));
        assert!(!prompt.contains("{language}"));
    }

    #[test]
    fn test_select_prompt_live_meeting_injects_language() {
        let config = PolishConfig {
            reasoning_effort: None,
            prompt_type: Some("live_meeting".to_string()),
            language_code: "da".to_string(),
        };
        let prompt = select_prompt(&config);
        assert!(prompt.contains("The output MUST be in Danish"));
        assert!(prompt.contains("## Summary"));
    }
}
