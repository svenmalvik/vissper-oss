//! User preferences storage
//!
//! Handles saving and loading user preferences to a JSON file
//! in the application support directory.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info};

/// AI provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AiProvider {
    #[default]
    Azure,
    OpenAI,
}

impl fmt::Display for AiProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiProvider::Azure => write!(f, "Azure OpenAI"),
            AiProvider::OpenAI => write!(f, "OpenAI"),
        }
    }
}

/// User preferences
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Preferences {
    /// AI provider selection (Azure OpenAI or OpenAI)
    /// Defaults to Azure for backward compatibility
    pub ai_provider: Option<AiProvider>,
    /// Custom transcript storage location (None = use default)
    pub transcript_location: Option<PathBuf>,
    /// Custom screenshot storage location (None = use default)
    pub screenshot_location: Option<PathBuf>,
    /// Language code for transcription (e.g., "en", "no", "da", "fi", "de")
    /// Defaults to "en" (English) if not set
    pub language_code: Option<String>,
    /// Last time version check was performed (ISO 8601 timestamp)
    pub last_version_check: Option<String>,
    /// Latest known version from remote (cached)
    pub latest_known_version: Option<String>,
    /// Download URL for the latest known version
    pub latest_download_url: Option<String>,
    /// Overlay transparency (0.3 to 1.0, defaults to 0.95)
    pub overlay_transparency: Option<f64>,
    /// Background mode (true = dark, false = light, defaults to true)
    pub is_dark_mode: Option<bool>,
}

/// Get the preferences file path
fn preferences_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("Vissper").join("preferences.json"))
}

/// Load preferences from disk
///
/// Returns default preferences if the file doesn't exist or can't be read
pub(crate) fn load_preferences() -> Preferences {
    let Some(path) = preferences_path() else {
        return Preferences::default();
    };

    if !path.exists() {
        return Preferences::default();
    }

    match fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(prefs) => prefs,
            Err(e) => {
                error!("Failed to parse preferences: {}", e);
                Preferences::default()
            }
        },
        Err(e) => {
            error!("Failed to read preferences file: {}", e);
            Preferences::default()
        }
    }
}

/// Save preferences to disk
pub(crate) fn save_preferences(prefs: &Preferences) -> Result<(), PreferencesError> {
    let path = preferences_path().ok_or(PreferencesError::NoConfigDir)?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
            info!("Created preferences directory: {:?}", parent);
        }
    }

    let json = serde_json::to_string_pretty(prefs)?;
    fs::write(&path, json)?;
    info!("Saved preferences to: {:?}", path);

    Ok(())
}

/// Get the custom transcript location, if set
pub(crate) fn get_transcript_location() -> Option<PathBuf> {
    load_preferences().transcript_location
}

/// Set a custom transcript location
pub(crate) fn set_transcript_location(path: Option<PathBuf>) -> Result<(), PreferencesError> {
    let mut prefs = load_preferences();
    prefs.transcript_location = path;
    save_preferences(&prefs)
}

/// Get the default transcript location path for display
pub(crate) fn default_transcript_location() -> Option<PathBuf> {
    dirs::document_dir().map(|d| d.join("Vissper").join("transcripts"))
}

/// Get the custom screenshot location, if set
pub(crate) fn get_screenshot_location() -> Option<PathBuf> {
    load_preferences().screenshot_location
}

/// Set a custom screenshot location
pub(crate) fn set_screenshot_location(path: Option<PathBuf>) -> Result<(), PreferencesError> {
    let mut prefs = load_preferences();
    prefs.screenshot_location = path;
    save_preferences(&prefs)
}

/// Get the default screenshot location path
pub(crate) fn default_screenshot_location() -> Option<PathBuf> {
    dirs::document_dir().map(|d| d.join("Vissper").join("screenshots"))
}

/// Get the language code for transcription
/// Returns "en" (English) if not set
pub(crate) fn get_language_code() -> String {
    load_preferences()
        .language_code
        .unwrap_or_else(|| "en".to_string())
}

/// Set the language code for transcription
pub(crate) fn set_language_code(code: &str) -> Result<(), PreferencesError> {
    let mut prefs = load_preferences();
    prefs.language_code = Some(code.to_string());
    save_preferences(&prefs)
}

/// Get the selected AI provider
/// Returns Azure (default) for backward compatibility if not set
pub(crate) fn get_ai_provider() -> AiProvider {
    load_preferences().ai_provider.unwrap_or_default()
}

/// Set the AI provider
pub(crate) fn set_ai_provider(provider: AiProvider) -> Result<(), PreferencesError> {
    let mut prefs = load_preferences();
    prefs.ai_provider = Some(provider);
    save_preferences(&prefs)
}

/// Default overlay transparency value (95%)
const DEFAULT_OVERLAY_TRANSPARENCY: f64 = 0.95;

/// Default dark mode setting (dark)
const DEFAULT_IS_DARK_MODE: bool = true;

/// Get the overlay transparency setting
/// Returns 0.95 (95%) if not set
pub(crate) fn get_overlay_transparency() -> f64 {
    load_preferences()
        .overlay_transparency
        .unwrap_or(DEFAULT_OVERLAY_TRANSPARENCY)
}

/// Set the overlay transparency setting
pub(crate) fn set_overlay_transparency(value: f64) -> Result<(), PreferencesError> {
    let mut prefs = load_preferences();
    prefs.overlay_transparency = Some(value.clamp(0.3, 1.0));
    save_preferences(&prefs)
}

/// Get the dark mode setting
/// Returns true (dark mode) if not set
pub(crate) fn get_is_dark_mode() -> bool {
    load_preferences()
        .is_dark_mode
        .unwrap_or(DEFAULT_IS_DARK_MODE)
}

/// Set the dark mode setting
pub(crate) fn set_is_dark_mode(is_dark: bool) -> Result<(), PreferencesError> {
    let mut prefs = load_preferences();
    prefs.is_dark_mode = Some(is_dark);
    save_preferences(&prefs)
}

/// Check if enough time has elapsed to perform a version check
///
/// Returns true if:
/// - Version has never been checked
/// - More than 24 hours have elapsed since last check
/// - Last check timestamp is invalid
pub(crate) fn should_check_for_updates() -> bool {
    let prefs = load_preferences();
    match prefs.last_version_check {
        None => true, // Never checked
        Some(last_check_str) => match chrono::DateTime::parse_from_rfc3339(&last_check_str) {
            Ok(last_check) => {
                let now = chrono::Utc::now();
                let elapsed = now.signed_duration_since(last_check);
                elapsed.num_hours() >= 24
            }
            Err(_) => {
                error!("Failed to parse last_version_check timestamp, will check anyway");
                true
            }
        },
    }
}

/// Update the last version check timestamp to now
pub(crate) fn update_version_check_time() -> Result<(), PreferencesError> {
    let mut prefs = load_preferences();
    prefs.last_version_check = Some(chrono::Utc::now().to_rfc3339());
    save_preferences(&prefs)
}

/// Get the latest known version from cache
pub(crate) fn get_latest_known_version() -> Option<String> {
    load_preferences()
        .latest_known_version
        .filter(|v| !v.is_empty())
}

/// Set the latest known version in cache
pub(crate) fn set_latest_known_version(version: &str) -> Result<(), PreferencesError> {
    let mut prefs = load_preferences();
    prefs.latest_known_version = Some(version.to_string());
    save_preferences(&prefs)
}

/// Get the latest known download URL from cache
pub(crate) fn get_latest_download_url() -> Option<String> {
    load_preferences()
        .latest_download_url
        .filter(|v| !v.is_empty())
}

/// Set the latest known download URL in cache
pub(crate) fn set_latest_download_url(url: &str) -> Result<(), PreferencesError> {
    let mut prefs = load_preferences();
    prefs.latest_download_url = Some(url.to_string());
    save_preferences(&prefs)
}

/// Preferences errors
#[derive(Debug, thiserror::Error)]
pub(crate) enum PreferencesError {
    #[error("Could not find config directory")]
    NoConfigDir,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_preferences() {
        let prefs = Preferences::default();
        assert!(prefs.ai_provider.is_none());
        assert!(prefs.transcript_location.is_none());
        assert!(prefs.screenshot_location.is_none());
        assert!(prefs.language_code.is_none());
    }

    #[test]
    fn test_ai_provider_default() {
        // Default should be Azure for backward compatibility
        assert_eq!(AiProvider::default(), AiProvider::Azure);
    }

    #[test]
    fn test_ai_provider_display() {
        assert_eq!(format!("{}", AiProvider::Azure), "Azure OpenAI");
        assert_eq!(format!("{}", AiProvider::OpenAI), "OpenAI");
    }

    #[test]
    fn test_preferences_path() {
        let path = preferences_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.ends_with("Vissper/preferences.json"));
    }

    #[test]
    fn test_default_screenshot_location() {
        let path = default_screenshot_location();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.ends_with("Vissper/screenshots"));
    }
}
