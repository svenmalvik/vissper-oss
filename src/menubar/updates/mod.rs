//! Menu bar UI update functions
//!
//! Thread-safe functions for updating the menu bar state and appearance.

mod app_update;
mod language;
mod state;

pub use app_update::{hide_update_available, show_update_available};
pub use language::set_language;
pub use state::{set_azure_credentials, set_processing, set_recording};

use objc2_foundation::MainThreadMarker;
use std::sync::atomic::Ordering;

use super::icons;
use super::{APP_STATE, MENU_BAR};

/// Update the menu bar UI based on current state
pub(super) fn update_ui() {
    let Some(state) = APP_STATE.get() else {
        return;
    };
    let Some(menu_bar) = MENU_BAR.get() else {
        return;
    };
    let Ok(inner) = menu_bar.lock() else {
        return;
    };

    let is_recording = state.is_recording.load(Ordering::SeqCst);
    let is_processing = state.is_processing.load(Ordering::SeqCst);
    let has_azure_credentials = state.has_azure_credentials.load(Ordering::SeqCst);

    // Update icon
    if let Some(mtm) = MainThreadMarker::new() {
        icons::set_icon(&inner.status_item, is_recording, is_processing, mtm);
    }

    // Update recording item
    if is_recording {
        let title_str = objc2_foundation::NSString::from_str("Stop Recording");
        unsafe {
            inner.recording_item.setTitle(&title_str);
            inner.recording_item.setSubmenu(Some(&inner.stop_submenu));
            inner.recording_item.setEnabled(true);
        }
    } else {
        let title_str = objc2_foundation::NSString::from_str("Start Recording");
        unsafe {
            inner.recording_item.setTitle(&title_str);
            inner.recording_item.setSubmenu(None);
            inner.recording_item.setEnabled(has_azure_credentials);
        }
    }

    // These items are always enabled in OSS version
    unsafe {
        inner.settings_item.setEnabled(true);
        inner.show_window_item.setEnabled(true);
        inner.screenshots_item.setEnabled(true);
        inner.screenshot_fullscreen_item.setEnabled(true);
        inner.screenshot_region_item.setEnabled(true);
        inner.languages_item.setEnabled(true);
    }
}

/// Dispatch UI update to main thread if needed
pub(super) fn dispatch_ui_update() {
    if MainThreadMarker::new().is_some() {
        update_ui();
    } else {
        dispatch::Queue::main().exec_async(|| {
            update_ui();
        });
    }
}
