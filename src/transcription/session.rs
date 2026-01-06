//! Transcription session state management

/// Accumulated transcription session data
#[derive(Debug, Default, Clone)]
pub struct TranscriptionSession {
    /// All committed transcript segments
    pub committed_segments: Vec<String>,
    /// Current partial transcript (if any)
    pub partial_transcript: Option<String>,
    /// Flag to indicate recording was manually stopped (not connection lost)
    /// Used to prevent ConnectionLost events from overwriting polished transcript
    pub manually_stopped: bool,
}

impl TranscriptionSession {
    /// Get the full transcript text
    pub fn full_transcript(&self) -> String {
        self.committed_segments.join(" ")
    }

    /// Insert a screenshot reference at the current position in the transcript
    ///
    /// The screenshot is inserted after all currently committed segments.
    /// Note: Any partial transcript will appear after the screenshot in the live view,
    /// but will be correctly positioned when the STT service commits it.
    ///
    /// # Arguments
    /// * `relative_path` - The relative path to the screenshot (e.g., "screenshots/screenshot-2025-12-11-14-30-45.png")
    pub fn insert_screenshot(&mut self, relative_path: &str) {
        let markdown_ref = format!("\n\n![Screenshot]({})\n\n", relative_path);
        self.committed_segments.push(markdown_ref);
    }
}
