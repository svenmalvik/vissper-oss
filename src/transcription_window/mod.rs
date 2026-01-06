//! Transcription Window implementation using objc2
//!
//! This module provides a transparent, borderless overlay window for displaying
//! real-time transcription text at the bottom center of the screen.

mod api;
mod components;
mod controls;
mod delegates;
mod markdown;
mod objc_utils;
mod state;
mod window;

use block2::RcBlock;
use objc2::msg_send;
use objc2_foundation::{MainThreadMarker, NSOperationQueue};
use std::sync::Mutex;
use tracing::info;

// Re-export for crate use
pub(crate) use state::{TabType, WindowCallbacks};

use state::{TRANSCRIPTION_WINDOW, WINDOW_CALLBACKS};

/// Transcription window manager
pub(crate) struct TranscriptionWindow;

impl TranscriptionWindow {
    /// Initialize window callbacks (must be called before show)
    pub(crate) fn init(callbacks: WindowCallbacks) {
        WINDOW_CALLBACKS.set(callbacks).ok();
    }

    /// Load appearance preferences from persistent storage.
    ///
    /// Call this at app startup to restore saved transparency and dark mode settings.
    pub(crate) fn load_appearance_preferences() {
        state::load_appearance_preferences();
    }

    /// Show the transcription window (creates it if not already created)
    /// Dispatches to main thread if called from a background thread.
    pub(crate) fn show() {
        info!("Opening transcription window");

        // If already on main thread, create window directly
        if let Some(mtm) = MainThreadMarker::new() {
            Self::show_on_main_thread(mtm);
            return;
        }

        // Not on main thread - dispatch to main queue
        info!("Dispatching window creation to main thread");
        let block = RcBlock::new(|| {
            if let Some(mtm) = MainThreadMarker::new() {
                Self::show_on_main_thread(mtm);
            }
        });

        unsafe {
            let queue = NSOperationQueue::mainQueue();
            let _: () = msg_send![&queue, addOperationWithBlock: &*block];
        }
    }

    /// Internal: show window on main thread (requires MainThreadMarker)
    fn show_on_main_thread(mtm: MainThreadMarker) {
        // Check if window already exists
        if let Some(inner) = TRANSCRIPTION_WINDOW.get() {
            if let Ok(inner) = inner.lock() {
                // Window exists, just show it
                inner.window.makeKeyAndOrderFront(None);
                return;
            }
        }

        // Create new window with all UI elements
        let inner = window::create_window(mtm);

        // Store in global state
        if TRANSCRIPTION_WINDOW.set(Mutex::new(inner)).is_err() {
            // Window was created by another thread, show that one instead
            if let Some(inner) = TRANSCRIPTION_WINDOW.get() {
                if let Ok(inner) = inner.lock() {
                    inner.window.makeKeyAndOrderFront(None);
                }
            }
        }
    }

    /// Update the displayed transcription text (for active tab)
    #[allow(dead_code)]
    pub(crate) fn update_text(committed: &str, partial: Option<&str>, use_monospaced: bool) {
        api::update_text(committed, partial, use_monospaced);
    }

    /// Hide the transcription window
    #[allow(dead_code)]
    pub(crate) fn hide() {
        api::hide();
    }

    /// Clear the transcription text
    #[allow(dead_code)]
    pub(crate) fn clear() {
        api::clear();
    }

    /// Set the recording state (shows recording indicator with "Recording" text)
    pub(crate) fn set_recording_state(recording: bool) {
        api::set_recording_state(recording);
    }

    /// Set the recording type label to "Transcription"
    pub(crate) fn set_recording_type() {
        api::set_recording_type();
    }

    /// Set the processing state (shows indicator with "Processing" text)
    pub(crate) fn set_processing_state(processing: bool) {
        api::set_processing_state(processing);
    }

    /// Set window transparency (0.0 = fully transparent, 1.0 = fully opaque)
    #[allow(dead_code)]
    pub(crate) fn set_transparency(alpha: f64) {
        api::set_transparency(alpha);
    }

    /// Handle hide button click (called from delegate)
    pub(crate) fn handle_hide_action() {
        api::handle_hide_action();
    }

    /// Adjust transparency by delta (positive = more opaque, negative = more transparent)
    pub(crate) fn adjust_transparency(delta: f64) {
        api::adjust_transparency(delta);
    }

    /// Get the current transparency value (0.3 to 1.0)
    pub(crate) fn get_transparency() -> f64 {
        api::get_transparency()
    }

    /// Get the current dark mode state
    pub(crate) fn is_dark_mode() -> bool {
        api::is_dark_mode()
    }

    /// Set dark or light mode for the window background
    pub(crate) fn set_dark_mode(is_dark: bool) {
        api::set_dark_mode(is_dark);
    }

    /// Show the save button and store transcript for later saving
    pub(crate) fn show_save_button(transcript: String) {
        api::show_save_button(transcript);
    }

    /// Hide the save button
    pub(crate) fn hide_save_button() {
        api::hide_save_button();
    }

    /// Handle save file button click (called from delegate)
    pub(crate) fn handle_save_file_action() {
        api::handle_save_file_action();
    }

    /// Handle tab change from segmented control (called from delegate)
    pub(crate) fn handle_tab_change_action(selected_index: isize) {
        api::handle_tab_change(selected_index);
    }

    /// Switch to a specific tab
    pub(crate) fn switch_to_tab(tab: TabType) {
        api::switch_to_tab(tab);
    }

    /// Update live transcript content (Tab 1)
    pub(crate) fn update_live_text(committed: &str, partial: Option<&str>) {
        api::update_live_text(committed, partial);
    }

    /// Set polished content (Tab 2)
    pub(crate) fn set_polished_content(content: &str) {
        api::set_polished_content(content);
    }

    /// Set meeting notes content (Tab 3)
    pub(crate) fn set_meeting_notes_content(content: &str) {
        api::set_meeting_notes_content(content);
    }

    /// Get the current raw transcript for on-demand polishing
    #[allow(dead_code)]
    pub(crate) fn get_live_transcript() -> Option<String> {
        api::get_live_transcript()
    }

    /// Reset tab content (called when starting a new recording)
    pub(crate) fn reset_tabs() {
        api::reset_tabs();
    }
}
