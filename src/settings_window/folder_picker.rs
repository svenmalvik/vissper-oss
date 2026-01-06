//! Folder picker dialogs for the settings window.

use std::path::PathBuf;

use objc2::rc::Retained;
use objc2::{msg_send_id, ClassType};
use objc2_app_kit::NSOpenPanel;
use objc2_foundation::{MainThreadMarker, NSString};
use tracing::{error, info};

use crate::preferences;
use crate::settings_window::constants::NS_MODAL_RESPONSE_OK;

/// Location type for folder picker configuration.
pub(crate) enum LocationType {
    Transcript,
    Screenshot,
}

/// Show the folder picker dialog for selecting a location.
///
/// Returns the selected path if the user made a selection, or None if cancelled.
pub(crate) fn show_folder_picker(location_type: LocationType) -> Option<PathBuf> {
    let (message, current_location) = match location_type {
        LocationType::Transcript => (
            "Choose a folder for saving transcripts",
            preferences::get_transcript_location()
                .or_else(preferences::default_transcript_location),
        ),
        LocationType::Screenshot => (
            "Choose a folder for saving screenshots",
            preferences::get_screenshot_location()
                .or_else(preferences::default_screenshot_location),
        ),
    };

    let location_name = match location_type {
        LocationType::Transcript => "transcript",
        LocationType::Screenshot => "screenshot",
    };

    info!("Opening folder picker for {} location", location_name);

    let Some(mtm) = MainThreadMarker::new() else {
        error!("Not on main thread, cannot show folder picker");
        return None;
    };

    // SAFETY: NSOpenPanel::openPanel is safe to call on main thread
    let panel = unsafe { NSOpenPanel::openPanel(mtm) };

    // SAFETY: These are standard NSOpenPanel configuration calls
    unsafe {
        panel.setCanChooseFiles(false);
        panel.setCanChooseDirectories(true);
        panel.setAllowsMultipleSelection(false);
        panel.setMessage(Some(&NSString::from_str(message)));
        panel.setPrompt(Some(&NSString::from_str("Select")));

        // Set initial directory to current location if it exists
        if let Some(ref current) = current_location {
            if current.exists() {
                let url_string = format!("file://{}", current.display());
                let ns_url_string = NSString::from_str(&url_string);
                // SAFETY: NSURL class method with valid URL string
                let url: Option<Retained<objc2_foundation::NSURL>> =
                    msg_send_id![objc2_foundation::NSURL::class(), URLWithString: &*ns_url_string];
                if let Some(url) = url {
                    panel.setDirectoryURL(Some(&url));
                }
            }
        }
    }

    // SAFETY: runModal blocks until user dismisses the panel
    let response = unsafe { panel.runModal() };

    if response == NS_MODAL_RESPONSE_OK {
        // SAFETY: URLs() returns a valid NSArray after successful modal
        let urls = unsafe { panel.URLs() };
        if let Some(url) = urls.first() {
            // SAFETY: path() returns the file system path from a file URL
            if let Some(path_str) = unsafe { url.path() } {
                let path = PathBuf::from(path_str.to_string());
                info!("User selected {} location: {:?}", location_name, path);
                return Some(path);
            }
        }
    }

    None
}

/// Show folder picker for transcript location and save the selection.
pub(crate) fn choose_transcript_location() -> bool {
    if let Some(path) = show_folder_picker(LocationType::Transcript) {
        if let Err(e) = preferences::set_transcript_location(Some(path)) {
            error!("Failed to save transcript location: {}", e);
            return false;
        }
        return true;
    }
    false
}

/// Show folder picker for screenshot location and save the selection.
pub(crate) fn choose_screenshot_location() -> bool {
    if let Some(path) = show_folder_picker(LocationType::Screenshot) {
        if let Err(e) = preferences::set_screenshot_location(Some(path)) {
            error!("Failed to save screenshot location: {}", e);
            return false;
        }
        return true;
    }
    false
}

/// Reset transcript location to default.
pub(crate) fn reset_transcript_location() -> bool {
    info!("Resetting transcript location to default");
    if let Err(e) = preferences::set_transcript_location(None) {
        error!("Failed to reset transcript location: {}", e);
        return false;
    }
    true
}

/// Reset screenshot location to default.
pub(crate) fn reset_screenshot_location() -> bool {
    info!("Resetting screenshot location to default");
    if let Err(e) = preferences::set_screenshot_location(None) {
        error!("Failed to reset screenshot location: {}", e);
        return false;
    }
    true
}
