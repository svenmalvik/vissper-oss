//! Path and folder picker actions.

use objc2_foundation::NSString;

use super::super::{folder_picker, path_utils, SETTINGS_WINDOW};

/// Show the folder picker dialog for selecting transcript location.
pub(in crate::settings_window) fn show_folder_picker() {
    if folder_picker::choose_transcript_location() {
        update_transcript_path_label();
    }
}

/// Reset transcript location to default.
pub(in crate::settings_window) fn reset_transcript_location() {
    if folder_picker::reset_transcript_location() {
        update_transcript_path_label();
    }
}

/// Update the transcript path label with the current location.
pub(in crate::settings_window) fn update_transcript_path_label() {
    if let Some(inner) = SETTINGS_WINDOW.get() {
        if let Ok(inner) = inner.lock() {
            let display_path = path_utils::get_transcript_display_path();
            unsafe {
                inner
                    .transcript_path_label
                    .setStringValue(&NSString::from_str(&display_path));
            }
        }
    }
}

/// Show the folder picker dialog for selecting screenshot location.
pub(in crate::settings_window) fn show_screenshot_folder_picker() {
    if folder_picker::choose_screenshot_location() {
        update_screenshot_path_label();
    }
}

/// Reset screenshot location to default.
pub(in crate::settings_window) fn reset_screenshot_location() {
    if folder_picker::reset_screenshot_location() {
        update_screenshot_path_label();
    }
}

/// Update the screenshot path label with the current location.
pub(in crate::settings_window) fn update_screenshot_path_label() {
    if let Some(inner) = SETTINGS_WINDOW.get() {
        if let Ok(inner) = inner.lock() {
            let display_path = path_utils::get_screenshot_display_path();
            unsafe {
                inner
                    .screenshot_path_label
                    .setStringValue(&NSString::from_str(&display_path));
            }
        }
    }
}
