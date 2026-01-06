//! Window appearance and visibility operations

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, msg_send_id};
use objc2_app_kit::NSColor;
use objc2_foundation::{NSRange, NSString};
use std::sync::atomic::Ordering;
use tracing::{error, info, warn};

use super::dispatch_to_main;
use crate::transcription_window::objc_utils;
use crate::transcription_window::state::{
    CURRENT_TRANSPARENCY, IS_DARK_MODE, TRANSCRIPTION_WINDOW, WINDOW_CALLBACKS,
};

/// Hide the transcription window.
///
/// Removes the window from screen without destroying it.
pub(crate) fn hide() {
    if let Some(inner) = TRANSCRIPTION_WINDOW.get() {
        if let Ok(inner) = inner.lock() {
            inner.window.orderOut(None);
            info!("Transcription window hidden");
        } else {
            error!("Failed to acquire transcription window lock in hide");
        }
    }
}

/// Handle hide button click.
///
/// Hides the window and invokes the on_hide callback.
pub(crate) fn handle_hide_action() {
    info!("Hide button clicked");
    hide();
    if let Some(callbacks) = WINDOW_CALLBACKS.get() {
        (callbacks.on_hide)();
    }
}

/// Set background transparency.
///
/// # Arguments
/// * `alpha` - Transparency value from 0.0 (fully transparent) to 1.0 (fully opaque).
///   Values are clamped to the range [0.3, 1.0].
///
/// This adjusts only the background's transparency, not the content.
/// Text, buttons, and other UI elements remain fully opaque and readable.
pub(crate) fn set_transparency(alpha: f64) {
    let alpha = alpha.clamp(0.3, 1.0);
    let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in set_transparency");
            return;
        };

        // Set background color with adjustable alpha
        // This only affects the window background, not the content (text, buttons)
        unsafe {
            let base = if is_dark { 0.1 } else { 0.95 };
            let bg_color = NSColor::colorWithRed_green_blue_alpha(base, base, base, alpha);
            inner.window.setBackgroundColor(Some(&bg_color));
        }
    });

    dispatch_to_main(&block);
}

/// Adjust transparency by delta.
///
/// # Arguments
/// * `delta` - Change in transparency (positive = more opaque, negative = more transparent).
///
/// The result is clamped to the range [0.3, 1.0].
pub(crate) fn adjust_transparency(delta: f64) {
    let current = CURRENT_TRANSPARENCY.load(Ordering::SeqCst) as f64 / 100.0;
    let new_value = (current + delta).clamp(0.3, 1.0);
    CURRENT_TRANSPARENCY.store((new_value * 100.0) as u32, Ordering::SeqCst);
    set_transparency(new_value);
}

/// Get the current transparency value.
///
/// Returns a value between 0.3 (30%) and 1.0 (100%).
pub(crate) fn get_transparency() -> f64 {
    CURRENT_TRANSPARENCY.load(Ordering::SeqCst) as f64 / 100.0
}

/// Get the current dark mode state.
///
/// Returns `true` if dark mode is active, `false` for light mode.
pub(crate) fn is_dark_mode() -> bool {
    IS_DARK_MODE.load(Ordering::SeqCst)
}

/// Set dark or light mode for the window.
///
/// Updates the window appearance, material, and text colors across all text views.
/// With NSVisualEffectView, this changes both the appearance and material for proper theming.
///
/// # Arguments
/// * `is_dark` - `true` for dark mode (dark appearance, light text),
///   `false` for light mode (light appearance, dark text).
pub(crate) fn set_dark_mode(is_dark: bool) {
    IS_DARK_MODE.store(is_dark, Ordering::SeqCst);

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in set_dark_mode");
            return;
        };

        // SAFETY: All msg_send calls are to valid NSWindow, NSAppearance, NSVisualEffectView, and NSTextView objects
        unsafe {
            // Set window appearance to force dark or light mode with graceful fallback
            let appearance_name = if is_dark {
                NSString::from_str("NSAppearanceNameVibrantDark")
            } else {
                NSString::from_str("NSAppearanceNameVibrantLight")
            };

            if let Some(appearance_class) = objc_utils::get_class_or_warn("NSAppearance") {
                let appearance: *mut AnyObject =
                    msg_send![appearance_class, appearanceNamed: &*appearance_name];
                if !appearance.is_null() {
                    let _: () = msg_send![&inner.window, setAppearance: appearance];
                }
            } else {
                warn!("NSAppearance not available - dark/light mode appearance may not apply correctly");
            }

            // Update window background color for the current mode
            // This maintains transparency while using the correct base color
            let current_alpha = CURRENT_TRANSPARENCY.load(Ordering::SeqCst) as f64 / 100.0;
            let base = if is_dark { 0.1 } else { 0.95 };
            let bg_color = NSColor::colorWithRed_green_blue_alpha(base, base, base, current_alpha);
            inner.window.setBackgroundColor(Some(&bg_color));

            let text_color = if is_dark {
                NSColor::whiteColor()
            } else {
                NSColor::blackColor()
            };

            // Update all three text views
            for text_view in [
                &inner.live_text_view,
                &inner.polished_text_view,
                &inner.meeting_text_view,
            ] {
                let text_storage: *mut AnyObject = msg_send![text_view, textStorage];
                if !text_storage.is_null() {
                    let length: usize = msg_send![text_storage, length];
                    if length > 0 {
                        let full_range = NSRange::new(0, length);
                        let color_attr_name = NSString::from_str("NSColor");
                        let _: () = msg_send![
                            text_storage,
                            addAttribute: &*color_attr_name,
                            value: &*text_color,
                            range: full_range
                        ];
                    }
                }
            }

            // Update header label color (muted but visible)
            let header_color = if is_dark {
                NSColor::colorWithRed_green_blue_alpha(0.6, 0.6, 0.6, 1.0)
            } else {
                NSColor::colorWithRed_green_blue_alpha(0.3, 0.3, 0.3, 1.0)
            };
            inner.recording_type_label.setTextColor(Some(&header_color));

            // Update save button text color
            let save_button_color = if is_dark {
                NSColor::colorWithRed_green_blue_alpha(0.55, 0.55, 0.55, 1.0)
            } else {
                NSColor::colorWithRed_green_blue_alpha(0.35, 0.35, 0.35, 1.0)
            };
            let attr_title: *mut AnyObject = msg_send![&inner.save_button, attributedTitle];
            if !attr_title.is_null() {
                let mutable_attr: Retained<AnyObject> = msg_send_id![attr_title, mutableCopy];
                let length: usize = msg_send![&mutable_attr, length];
                if length > 0 {
                    let range = NSRange::new(0, length);
                    let color_key = NSString::from_str("NSColor");
                    let _: () = msg_send![&mutable_attr, addAttribute: &*color_key value: &*save_button_color range: range];
                    let _: () = msg_send![&inner.save_button, setAttributedTitle: &*mutable_attr];
                }
            }
            // Also update the content tint color for the SF Symbol icon
            let _: () = msg_send![&inner.save_button, setContentTintColor: &*save_button_color];

            // Update close button text color
            let close_button_color = if is_dark {
                NSColor::colorWithRed_green_blue_alpha(0.55, 0.55, 0.55, 1.0)
            } else {
                NSColor::colorWithRed_green_blue_alpha(0.35, 0.35, 0.35, 1.0)
            };
            let attr_title: *mut AnyObject = msg_send![&inner.hide_button, attributedTitle];
            if !attr_title.is_null() {
                let mutable_attr: Retained<AnyObject> = msg_send_id![attr_title, mutableCopy];
                let length: usize = msg_send![&mutable_attr, length];
                if length > 0 {
                    let range = NSRange::new(0, length);
                    let color_key = NSString::from_str("NSColor");
                    let _: () = msg_send![&mutable_attr, addAttribute: &*color_key value: &*close_button_color range: range];
                    let _: () = msg_send![&inner.hide_button, setAttributedTitle: &*mutable_attr];
                }
            }
        }
    });

    dispatch_to_main(&block);
}
