//! Path formatting utilities for the settings window.

use std::path::PathBuf;

use crate::preferences;

/// Format a path for display, replacing home directory with `~`.
pub(crate) fn format_path_for_display(path: Option<&PathBuf>) -> String {
    match path {
        Some(p) => {
            if let Some(home) = dirs::home_dir() {
                if let Ok(stripped) = p.strip_prefix(&home) {
                    return format!("~/{}", stripped.display());
                }
            }
            p.display().to_string()
        }
        None => "~/Documents/Vissper/transcripts".to_string(),
    }
}

/// Get the display path for the current transcript location.
pub(crate) fn get_transcript_display_path() -> String {
    format_path_for_display(
        preferences::get_transcript_location()
            .or_else(preferences::default_transcript_location)
            .as_ref(),
    )
}

/// Get the display path for the current screenshot location.
pub(crate) fn get_screenshot_display_path() -> String {
    format_path_for_display(
        preferences::get_screenshot_location()
            .or_else(preferences::default_screenshot_location)
            .as_ref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_path_with_home_prefix() {
        if let Some(home) = dirs::home_dir() {
            let path = home.join("Documents").join("test");
            let result = format_path_for_display(Some(&path));
            assert!(result.starts_with("~/"));
            assert!(result.contains("Documents/test"));
        }
    }

    #[test]
    fn test_format_path_without_home_prefix() {
        let path = PathBuf::from("/tmp/transcripts");
        let result = format_path_for_display(Some(&path));
        assert_eq!(result, "/tmp/transcripts");
    }

    #[test]
    fn test_format_path_none() {
        let result = format_path_for_display(None);
        assert_eq!(result, "~/Documents/Vissper/transcripts");
    }
}
