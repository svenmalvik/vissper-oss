//! Local storage module for saving transcripts
//!
//! Handles saving transcripts to the user's Documents folder,
//! or a custom location if configured in preferences.

use crate::preferences;
use chrono::Local;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tracing::info;

/// Get the Vissper transcripts directory
///
/// Returns the custom location from preferences if set,
/// otherwise returns the default location in Documents.
pub(crate) fn transcripts_dir() -> Option<PathBuf> {
    // Check for custom location in preferences first
    if let Some(custom) = preferences::get_transcript_location() {
        return Some(custom);
    }
    // Fall back to default location
    dirs::document_dir().map(|d| d.join("Vissper").join("transcripts"))
}

/// Ensure the transcripts directory exists
#[allow(dead_code)]
pub(crate) fn ensure_transcripts_dir() -> Result<PathBuf, StorageError> {
    let dir = transcripts_dir().ok_or(StorageError::NoDocumentsDir)?;

    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| StorageError::CreateDirectory {
            path: dir.clone(),
            source: e,
        })?;
        info!("Created transcripts directory: {:?}", dir);
    }

    Ok(dir)
}

/// Save a transcript to a file
///
/// Returns the path to the saved file
#[allow(dead_code)]
pub(crate) fn save_transcript(transcript: &str) -> Result<PathBuf, StorageError> {
    if transcript.trim().is_empty() {
        return Err(StorageError::EmptyTranscript);
    }

    let dir = ensure_transcripts_dir()?;

    // Generate filename with timestamp
    let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S");
    let filename = format!("transcript-{}.md", timestamp);
    let filepath = dir.join(&filename);

    // Write transcript to file
    let mut file = fs::File::create(&filepath).map_err(|e| StorageError::CreateFile {
        path: filepath.clone(),
        source: e,
    })?;

    file.write_all(transcript.as_bytes())
        .map_err(|e| StorageError::WriteFile {
            path: filepath.clone(),
            source: e,
        })?;

    file.flush().map_err(|e| StorageError::WriteFile {
        path: filepath.clone(),
        source: e,
    })?;

    info!("Saved transcript to: {:?}", filepath);
    Ok(filepath)
}

/// Storage errors with contextual information
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub(crate) enum StorageError {
    #[error("Could not find Documents directory")]
    NoDocumentsDir,

    #[error("Transcript is empty")]
    EmptyTranscript,

    #[error("Failed to create directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to create file {path}: {source}")]
    CreateFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write to file {path}: {source}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use crate::preferences;

    #[test]
    fn test_default_transcripts_dir() {
        // Test the default location (not affected by user preferences)
        let dir = preferences::default_transcript_location();
        assert!(dir.is_some());
        let path = dir.unwrap();
        assert!(path.ends_with("Vissper/transcripts"));
    }
}
