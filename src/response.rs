//! Polishing configuration for transcript processing
//!
//! Defines configuration options for transcript polishing using Azure OpenAI.

use crate::preferences;
use serde::{Deserialize, Serialize};

/// Configuration for transcript polishing
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct PolishConfig {
    /// Reasoning effort level (e.g., "none", "low", "medium", "high")
    pub(crate) reasoning_effort: Option<String>,
    /// Prompt type to use ("default" or "live_meeting")
    pub(crate) prompt_type: Option<String>,
    /// Language code for output (e.g., "en", "no", "da")
    pub(crate) language_code: String,
}

/// Convert a language code to its full name for use in prompts
pub(crate) fn language_code_to_name(code: &str) -> &str {
    match code {
        "en" => "English",
        "no" => "Norwegian",
        "da" => "Danish",
        "fi" => "Finnish",
        "de" => "German",
        _ => code, // Return code itself for unknown languages
    }
}

impl PolishConfig {
    /// Create a config for basic transcript polishing
    /// Uses "none" reasoning for fast, straightforward copyediting
    pub fn basic_polish() -> Self {
        Self {
            reasoning_effort: Some("none".to_string()),
            prompt_type: None,
            language_code: preferences::get_language_code(),
        }
    }

    /// Create a config for live meeting recording
    /// Uses "low" reasoning for comprehensive meeting analysis
    pub fn live_meeting() -> Self {
        Self {
            reasoning_effort: Some("low".to_string()),
            prompt_type: Some("live_meeting".to_string()),
            language_code: preferences::get_language_code(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polish_config_basic_polish() {
        let config = PolishConfig::basic_polish();
        assert_eq!(config.reasoning_effort, Some("none".to_string()));
        assert!(config.prompt_type.is_none());
        assert!(!config.language_code.is_empty());
    }

    #[test]
    fn test_polish_config_live_meeting() {
        let config = PolishConfig::live_meeting();
        assert_eq!(config.reasoning_effort, Some("low".to_string()));
        assert_eq!(config.prompt_type, Some("live_meeting".to_string()));
        assert!(!config.language_code.is_empty());
    }

    #[test]
    fn test_language_code_to_name() {
        assert_eq!(language_code_to_name("en"), "English");
        assert_eq!(language_code_to_name("no"), "Norwegian");
        assert_eq!(language_code_to_name("da"), "Danish");
        assert_eq!(language_code_to_name("fi"), "Finnish");
        assert_eq!(language_code_to_name("de"), "German");
        assert_eq!(language_code_to_name("unknown"), "unknown");
    }
}
