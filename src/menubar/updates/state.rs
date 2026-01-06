//! State setter functions for menu bar updates
//!
//! Thread-safe functions for updating recording, processing, and Azure credentials states.

use std::sync::atomic::Ordering;

use super::dispatch_ui_update;
use crate::menubar::APP_STATE;

/// Set Azure credentials state (thread-safe)
///
/// When Azure credentials are available, recording is enabled.
pub fn set_azure_credentials(available: bool) {
    if let Some(state) = APP_STATE.get() {
        state
            .has_azure_credentials
            .store(available, Ordering::SeqCst);
    }

    dispatch_ui_update();
}

/// Set recording state (thread-safe)
pub fn set_recording(recording: bool) {
    if let Some(state) = APP_STATE.get() {
        state.is_recording.store(recording, Ordering::SeqCst);
    }

    dispatch_ui_update();
}

/// Set processing state (thread-safe)
pub fn set_processing(processing: bool) {
    if let Some(state) = APP_STATE.get() {
        state.is_processing.store(processing, Ordering::SeqCst);
    }

    dispatch_ui_update();
}
