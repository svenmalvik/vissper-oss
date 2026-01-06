//! Application callback setup
//!
//! Configures menu bar and hotkey callbacks for the application.

mod hotkeys;

use crate::menubar::MenuCallbacks;
use crate::recording::{self, RecordingSession};
use crate::settings_window;
use crate::transcription_window;
use std::sync::{Arc, Mutex};
use tracing::info;

pub(crate) use hotkeys::setup_hotkey_callbacks;

/// Configuration for creating callbacks
pub(crate) struct CallbackConfig {
    pub(crate) recording_state: Arc<Mutex<Option<RecordingSession>>>,
}

/// Create menu bar callbacks
pub(crate) fn create_menu_callbacks(config: &CallbackConfig) -> MenuCallbacks {
    let recording_state_start = config.recording_state.clone();
    let recording_state_no_polish = config.recording_state.clone();
    let recording_state_basic_polish = config.recording_state.clone();
    let recording_state_meeting_notes = config.recording_state.clone();
    let recording_state_screenshot = config.recording_state.clone();
    let recording_state_region_screenshot = config.recording_state.clone();

    MenuCallbacks {
        on_start_recording: Box::new(move || {
            info!("Starting recording...");
            recording::start_recording(recording_state_start.clone(), true);
        }),

        on_stop_no_polish: Box::new(move || {
            info!("Stopping recording (no polishing)...");
            recording::stop_recording_no_polish(recording_state_no_polish.clone());
        }),

        on_stop_basic_polish: Box::new(move || {
            info!("Stopping recording (basic polishing)...");
            recording::stop_recording(recording_state_basic_polish.clone());
        }),

        on_stop_meeting_notes: Box::new(move || {
            info!("Stopping recording (meeting notes)...");
            recording::stop_live_meeting_recording(recording_state_meeting_notes.clone());
        }),

        on_show_window: Box::new(|| {
            info!("Show window clicked");
            transcription_window::TranscriptionWindow::show();
        }),

        on_screenshot: Box::new(move || {
            info!("Taking screenshot...");
            match crate::screenshot::capture_screenshot() {
                Ok(filename) => {
                    info!("Screenshot captured: {}", filename);
                    crate::screenshot_flash::ScreenshotFlash::show();

                    if let Ok(state) = recording_state_screenshot.lock() {
                        if let Some(ref session) = *state {
                            if let Ok(mut session_data) = session.session_data.lock() {
                                let relative_path = format!("screenshots/{}", filename);
                                session_data.insert_screenshot(&relative_path);
                                info!("Screenshot reference inserted into transcript");
                            }
                        } else {
                            info!("Screenshot saved but no active recording session");
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to capture screenshot: {}", e);
                }
            }
        }),

        on_region_screenshot: Box::new(move || {
            info!("Starting region screenshot selection...");
            crate::region_selection::RegionSelection::start(
                recording_state_region_screenshot.clone(),
            );
        }),

        on_settings: Box::new(|| {
            info!("Settings clicked");
            settings_window::SettingsWindow::show();
        }),

        on_quit: Box::new(|| {
            info!("Quitting application...");
            std::process::exit(0);
        }),

        on_update_available: Box::new(move || {
            info!("on_update_available callback triggered");
            match crate::version_check::get_download_url_from_cache() {
                Some(download_url) => {
                    info!("Found cached download URL: {}", download_url);
                    if let Err(e) = open::that(&download_url) {
                        tracing::error!("Failed to open download URL: {}", e);
                    } else {
                        info!("Successfully opened download URL");
                    }
                }
                None => {
                    tracing::warn!("No cached download URL found");
                }
            }
        }),
    }
}
