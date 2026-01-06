//! Helper functions for transcript polishing
//!
//! Contains UI state management and error handling helpers for polish operations.

use crate::menubar;
use crate::transcription_window::{self, TabType};

use super::clipboard::copy_to_clipboard;

/// Handle polish failure by falling back to raw transcript
pub(super) fn handle_polish_failure(transcript: &str, target_tab: TabType) {
    copy_to_clipboard(transcript);
    // Show raw transcript in the target tab
    match target_tab {
        TabType::BasicPolish => {
            let msg = format!("⚠️ Polishing failed. Raw transcript:\n\n{}", transcript);
            transcription_window::TranscriptionWindow::set_polished_content(&msg);
        }
        TabType::MeetingNotes => {
            let msg = format!(
                "⚠️ Meeting notes generation failed. Raw transcript:\n\n{}",
                transcript
            );
            transcription_window::TranscriptionWindow::set_meeting_notes_content(&msg);
        }
        TabType::Live => {
            transcription_window::TranscriptionWindow::update_live_text(transcript, None);
        }
    }
    transcription_window::TranscriptionWindow::switch_to_tab(target_tab);
    show_save_button(transcript.to_string());
    reset_processing_state();
}

/// Handle transcript too large error
pub(super) fn handle_transcript_too_large(
    transcript: &str,
    length: usize,
    max_length: usize,
    target_tab: TabType,
) {
    let display_text = format!(
        "⚠️ Transcript too large to process\n\nYour transcript is {} characters, but the maximum is {}.\n\nRaw transcript:\n{}",
        length, max_length, transcript
    );
    match target_tab {
        TabType::BasicPolish => {
            transcription_window::TranscriptionWindow::set_polished_content(&display_text);
        }
        TabType::MeetingNotes => {
            transcription_window::TranscriptionWindow::set_meeting_notes_content(&display_text);
        }
        TabType::Live => {
            transcription_window::TranscriptionWindow::update_live_text(&display_text, None);
        }
    }
    transcription_window::TranscriptionWindow::switch_to_tab(target_tab);
    copy_to_clipboard(transcript);
    show_save_button(transcript.to_string());
    reset_processing_state();
}

/// Reset processing state in UI
pub(super) fn reset_processing_state() {
    menubar::MenuBar::set_processing(false);
    transcription_window::TranscriptionWindow::set_processing_state(false);
}

/// Show save button to allow user to manually save the transcript
fn show_save_button(transcript: String) {
    if !transcript.trim().is_empty() {
        transcription_window::TranscriptionWindow::show_save_button(transcript);
    }
}

/// Set polished content in the appropriate tab
fn set_polished_content(content: &str, target_tab: TabType) {
    match target_tab {
        TabType::BasicPolish => {
            transcription_window::TranscriptionWindow::set_polished_content(content);
        }
        TabType::MeetingNotes => {
            transcription_window::TranscriptionWindow::set_meeting_notes_content(content);
        }
        TabType::Live => {}
    }
}

/// Handle successful polish result
pub(super) fn handle_polish_success(polished: String, target_tab: TabType) {
    set_polished_content(&polished, target_tab);
    transcription_window::TranscriptionWindow::switch_to_tab(target_tab);
    copy_to_clipboard(&polished);
    show_save_button(polished);
}

/// Handle generic polish error by showing raw transcript
pub(super) fn handle_polish_error(transcript: &str, target_tab: TabType) {
    set_polished_content(transcript, target_tab);
    transcription_window::TranscriptionWindow::switch_to_tab(target_tab);
    copy_to_clipboard(transcript);
    show_save_button(transcript.to_string());
}
