//! Menu bar icon handling
//!
//! Manages status bar icons for different application states.

use objc2::rc::Retained;
use objc2::{msg_send, msg_send_id};
use objc2_app_kit::{NSImage, NSStatusBarButton, NSStatusItem};
use objc2_foundation::{MainThreadMarker, NSData, NSSize};

// Embedded icons as PNG data (18x18 template images)

/// Idle icon - microphone outline (18x18 PNG)
const ICON_IDLE: &[u8] = include_bytes!("../../assets/icon_idle.png");

/// Recording icon - filled microphone (18x18 PNG)
const ICON_RECORDING: &[u8] = include_bytes!("../../assets/icon_recording.png");

/// Processing icon - orange microphone (18x18 PNG, not a template - fixed color)
const ICON_PROCESSING: &[u8] = include_bytes!("../../assets/icon_processing.png");

/// Set the menu bar icon based on recording/processing state
pub(super) fn set_icon(
    status_item: &NSStatusItem,
    is_recording: bool,
    is_processing: bool,
    mtm: MainThreadMarker,
) {
    let (icon_data, is_template) = if is_processing {
        // Processing icon is NOT a template - it should stay orange
        (ICON_PROCESSING, false)
    } else if is_recording {
        (ICON_RECORDING, false)
    } else {
        (ICON_IDLE, true)
    };

    let data = NSData::with_bytes(icon_data);

    let image = NSImage::initWithData(mtm.alloc(), &data);

    if let Some(image) = image {
        // Set as template image for proper dark/light mode support (except processing)
        unsafe { image.setTemplate(is_template) };

        // Set size for retina display
        unsafe { image.setSize(NSSize::new(18.0, 18.0)) };

        // Set on the button
        unsafe {
            let button: Option<Retained<NSStatusBarButton>> = msg_send_id![status_item, button];
            if let Some(button) = button {
                let _: () = msg_send![&button, setImage: &*image];
            }
        }
    }
}
