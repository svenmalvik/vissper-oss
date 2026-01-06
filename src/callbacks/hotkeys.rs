//! Hotkey callback setup
//!
//! Configures global hotkey handlers for recording control.

use crate::menubar::AppState;
use crate::recording;
use crate::screenshot;
use std::sync::Arc;
use tracing::{error, info};

use super::CallbackConfig;

/// Setup hotkey callbacks
pub(crate) fn setup_hotkey_callbacks(
    config: &CallbackConfig,
    app_state: Arc<AppState>,
    runtime_handle: tokio::runtime::Handle,
) {
    let recording_state = config.recording_state.clone();

    // Clone for each hotkey
    let recording_state_no_polish = recording_state.clone();
    let app_state_no_polish = app_state.clone();

    let recording_state_basic = recording_state.clone();
    let app_state_basic = app_state.clone();

    let recording_state_meeting = recording_state.clone();
    let app_state_meeting = app_state.clone();

    let recording_state_screenshot = recording_state.clone();
    let recording_state_region = recording_state.clone();

    let runtime_basic = runtime_handle.clone();
    let runtime_meeting = runtime_handle.clone();

    crate::hotkeys::start_hotkey_listener(
        // No polishing callback (Control + Space)
        Arc::new(move || {
            use std::sync::atomic::Ordering;

            let is_recording = app_state_no_polish.is_recording.load(Ordering::SeqCst);
            let recording_state = recording_state_no_polish.clone();

            runtime_handle.spawn(async move {
                if is_recording {
                    info!("Hotkey: Stopping recording (no polishing)");
                    recording::stop_recording_no_polish(recording_state);
                } else {
                    info!("Hotkey: Starting recording");
                    recording::start_recording(recording_state, true);
                }
            });
        }),
        // Basic polishing callback (Control + Shift + 1)
        Arc::new(move || {
            use std::sync::atomic::Ordering;

            let is_recording = app_state_basic.is_recording.load(Ordering::SeqCst);
            let recording_state = recording_state_basic.clone();

            runtime_basic.spawn(async move {
                if is_recording {
                    info!("Hotkey: Stopping recording (basic polishing)");
                    recording::stop_recording(recording_state);
                } else {
                    info!("Hotkey: Starting recording");
                    recording::start_recording(recording_state, true);
                }
            });
        }),
        // Meeting notes callback (Control + Shift + 2)
        Arc::new(move || {
            use std::sync::atomic::Ordering;

            let is_recording = app_state_meeting.is_recording.load(Ordering::SeqCst);
            let recording_state = recording_state_meeting.clone();

            runtime_meeting.spawn(async move {
                if is_recording {
                    info!("Hotkey: Stopping recording (meeting notes)");
                    recording::stop_live_meeting_recording(recording_state);
                } else {
                    info!(
                        "Hotkey: Control+Shift+2 only stops with meeting notes (use Control+Space or Control+Shift+1 to start)"
                    );
                }
            });
        }),
        // Screenshot callback (Control + Shift + 0)
        Arc::new(move || {
            info!("Hotkey: Taking screenshot");
            match screenshot::capture_screenshot() {
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
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to capture screenshot: {}", e);
                }
            }
        }),
        // Region screenshot callback (Control + Shift + 9)
        Arc::new(move || {
            info!("Hotkey: Region screenshot selection");
            crate::region_selection::RegionSelection::start(recording_state_region.clone());
        }),
    );
}
