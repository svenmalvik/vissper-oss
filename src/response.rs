//! Polishing configuration for transcript processing
//!
//! Defines configuration options for transcript polishing using Azure OpenAI.

use serde::{Deserialize, Serialize};

/// Configuration for transcript polishing
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct PolishConfig {
    /// Reasoning effort level (e.g., "none", "low", "medium", "high")
    pub(crate) reasoning_effort: Option<String>,
    /// Prompt type to use ("default" or "live_meeting")
    pub(crate) prompt_type: Option<String>,
}

impl PolishConfig {
    /// Create a config for basic transcript polishing
    /// Uses "none" reasoning for fast, straightforward copyediting
    pub fn basic_polish() -> Self {
        Self {
            reasoning_effort: Some("none".to_string()),
            prompt_type: None,
        }
    }

    /// Create a config for live meeting recording
    /// Uses "low" reasoning for comprehensive meeting analysis
    pub fn live_meeting() -> Self {
        Self {
            reasoning_effort: Some("low".to_string()),
            prompt_type: Some("live_meeting".to_string()),
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
    }

    #[test]
    fn test_polish_config_live_meeting() {
        let config = PolishConfig::live_meeting();
        assert_eq!(config.reasoning_effort, Some("low".to_string()));
        assert_eq!(config.prompt_type, Some("live_meeting".to_string()));
    }
}
