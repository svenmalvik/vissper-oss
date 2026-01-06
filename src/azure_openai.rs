//! Direct Azure OpenAI client for transcript polishing.
//!
//! This module provides a client that connects directly to Azure OpenAI,
//! bypassing the VIPS AI Gateway or Supabase edge functions. Users provide
//! their own Azure OpenAI credentials.

use crate::error::ResponseError;
use crate::keychain::AzureCredentials;
use crate::response::PolishConfig;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, instrument, warn};
use zeroize::Zeroize;

/// Maximum number of retry attempts for transient failures.
const MAX_RETRIES: u32 = 3;

/// Initial delay between retries (doubles with each attempt).
const INITIAL_RETRY_DELAY_MS: u64 = 1000;

/// Client for direct Azure OpenAI Responses API calls.
pub(crate) struct AzureOpenAIClient {
    endpoint_url: String,
    api_key: String,
    polish_deployment: String,
    client: reqwest::Client,
}

/// Request body for Azure OpenAI Responses API.
#[derive(Debug, Serialize)]
struct AzurePolishRequest {
    model: String,
    input: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<Reasoning>,
}

/// Message in the Azure OpenAI request.
#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Reasoning configuration for Azure OpenAI.
#[derive(Debug, Serialize)]
struct Reasoning {
    effort: String,
}

/// Response from Azure OpenAI Responses API.
#[derive(Debug, Deserialize)]
struct AzurePolishResponse {
    output: Vec<OutputItem>,
}

/// Output item in the response.
#[derive(Debug, Deserialize)]
struct OutputItem {
    #[serde(rename = "type")]
    item_type: String,
    #[serde(default)]
    content: Vec<ContentItem>,
}

/// Content item in the output.
#[derive(Debug, Deserialize)]
struct ContentItem {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

/// System prompt for basic transcript polishing.
const POLISH_PROMPT: &str = r#"You are an expert copy editor. Your task is to polish and copyedit the following transcript for improved readability and grammar while preserving the original meaning and tone. Fix any obvious transcription errors, improve punctuation, and ensure proper sentence structure. Do not add new content or change the meaning. Preserve the original language of the transcript - do not translate it.

IMPORTANT: The transcript may contain screenshot references in markdown image format like `![Screenshot](screenshots/filename.png)`. These must be preserved exactly as they appear, in their original positions within the transcript. Do not modify, remove, or relocate these screenshot references.

Return only the polished transcript without any additional commentary."#;

/// System prompt for live meeting recording.
const LIVE_MEETING_PROMPT: &str = r#"You are an expert meeting assistant. Your task is to analyze the following meeting transcript and generate a comprehensive, well-structured output. Preserve the original language of the transcript - do not translate it. All output sections should be in the same language as the transcript.

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

/// Select the appropriate prompt based on config
fn select_prompt(config: &PolishConfig) -> &'static str {
    match config.prompt_type.as_deref() {
        Some("live_meeting") => LIVE_MEETING_PROMPT,
        _ => POLISH_PROMPT,
    }
}

impl AzureOpenAIClient {
    /// Create a new Azure OpenAI client from credentials.
    pub(crate) fn new(creds: &AzureCredentials) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client for AzureOpenAIClient")?;

        Ok(Self {
            endpoint_url: creds.endpoint_url.clone(),
            api_key: creds.api_key.clone(),
            polish_deployment: creds.polish_deployment.clone(),
            client,
        })
    }

    /// Polish a transcript using Azure OpenAI Responses API.
    ///
    /// Sends the raw transcript to Azure OpenAI for copyediting and polishing.
    /// Includes retry logic for transient network failures.
    #[instrument(skip(self, transcript, config), fields(transcript_len = transcript.len()))]
    pub(crate) async fn polish_transcript(
        &self,
        transcript: &str,
        config: &PolishConfig,
    ) -> Result<String, ResponseError> {
        // For Azure, always use the configured deployment name
        // (config.model is for proxy backends that can route to different models)
        let model = self.polish_deployment.clone();

        let reasoning = config.reasoning_effort.as_ref().map(|effort| Reasoning {
            effort: effort.clone(),
        });

        let prompt = select_prompt(config);
        let request_body = AzurePolishRequest {
            model,
            input: vec![
                Message {
                    role: "developer".to_string(),
                    content: prompt.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: transcript.to_string(),
                },
            ],
            reasoning,
        };

        // Azure OpenAI Responses API
        // Try non-v1 format with api-version for Data Zone Standard deployments
        let endpoint = self.endpoint_url.trim_end_matches('/');
        let url = format!("{endpoint}/openai/responses?api-version=2025-04-01-preview");

        let mut last_error: Option<ResponseError> = None;
        let mut retry_delay = Duration::from_millis(INITIAL_RETRY_DELAY_MS);

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                warn!(
                    attempt = attempt,
                    max_retries = MAX_RETRIES,
                    delay_ms = retry_delay.as_millis(),
                    "Retrying Azure polish request after transient failure"
                );
                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2;
            }

            let result = self
                .client
                .post(&url)
                .header("api-key", &self.api_key)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            match result {
                Ok(response) => {
                    if response.status().is_success() {
                        let azure_response: AzurePolishResponse =
                            response.json().await.map_err(|e| {
                                ResponseError::InvalidResponse(format!(
                                    "Failed to parse Azure response: {}",
                                    e
                                ))
                            })?;

                        // Extract text from response
                        let polished_text = Self::extract_text(&azure_response)?;

                        if attempt > 0 {
                            info!(
                                attempt = attempt,
                                "Azure polish request succeeded after retry"
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

    /// Extract text from the Azure response structure.
    fn extract_text(response: &AzurePolishResponse) -> Result<String, ResponseError> {
        for output in &response.output {
            if output.item_type == "message" {
                for content in &output.content {
                    if content.content_type == "output_text" && !content.text.is_empty() {
                        return Ok(content.text.clone());
                    }
                }
            }
        }

        Err(ResponseError::InvalidResponse(
            "No text content in Azure response".into(),
        ))
    }

    /// Check if a reqwest error is retryable (transient).
    fn is_retryable_error(error: &reqwest::Error) -> bool {
        error.is_timeout() || error.is_connect() || error.is_request()
    }
}

impl Drop for AzureOpenAIClient {
    fn drop(&mut self) {
        // Clear API key from memory
        self.api_key.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_azure_polish_request_serialization() {
        let request = AzurePolishRequest {
            model: "gpt-5.1".to_string(),
            input: vec![
                Message {
                    role: "developer".to_string(),
                    content: "System prompt".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: "Hello world".to_string(),
                },
            ],
            reasoning: None,
        };

        let json = serde_json::to_string(&request).expect("Failed to serialize");
        assert!(json.contains("gpt-5.1"));
        assert!(json.contains("developer"));
        assert!(json.contains("Hello world"));
        assert!(!json.contains("reasoning"));
    }

    #[test]
    fn test_azure_polish_request_with_reasoning() {
        let request = AzurePolishRequest {
            model: "gpt-5.2".to_string(),
            input: vec![Message {
                role: "user".to_string(),
                content: "Test".to_string(),
            }],
            reasoning: Some(Reasoning {
                effort: "low".to_string(),
            }),
        };

        let json = serde_json::to_string(&request).expect("Failed to serialize");
        assert!(json.contains("reasoning"));
        assert!(json.contains("low"));
    }

    #[test]
    fn test_azure_response_deserialization() {
        let json = r#"{
            "output": [
                {
                    "type": "message",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Polished text here"
                        }
                    ]
                }
            ]
        }"#;

        let response: AzurePolishResponse =
            serde_json::from_str(json).expect("Failed to deserialize");
        let text = AzureOpenAIClient::extract_text(&response).expect("Failed to extract text");
        assert_eq!(text, "Polished text here");
    }
}
