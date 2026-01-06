//! Transparency control UI elements for the settings window.

use objc2::rc::Retained;
use objc2::sel;
use objc2_app_kit::{NSSlider, NSTextField, NSView};
use objc2_foundation::{CGFloat, MainThreadMarker, NSPoint, NSRect, NSSize};

use super::helpers::{create_section_label, create_slider, create_value_label};
use crate::settings_window::constants::PADDING;
use crate::settings_window::delegate::SettingsActionDelegate;
use crate::transcription_window::TranscriptionWindow;

/// Add transparency control UI elements to the content view.
///
/// Returns the slider and value label so they can be updated.
pub(crate) fn add_transparency_controls(
    mtm: MainThreadMarker,
    content_view: &NSView,
    delegate: &SettingsActionDelegate,
) -> (Retained<NSSlider>, Retained<NSTextField>) {
    // Get content view width for layout calculations
    let content_width = content_view.frame().size.width;

    let label_height: CGFloat = 20.0;
    let slider_width: CGFloat = 300.0;
    let slider_height: CGFloat = 21.0;
    let value_label_height: CGFloat = 16.0;

    // Section label at top of tab content
    let label_y: CGFloat = 330.0;
    let label_frame = NSRect::new(
        NSPoint::new(PADDING, label_y),
        NSSize::new(content_width - PADDING * 2.0, label_height),
    );
    let label = create_section_label(mtm, label_frame, "Overlay Transparency");

    // Slider centered below label
    let slider_y: CGFloat = 300.0;
    let slider_x = (content_width - slider_width) / 2.0;
    let slider_frame = NSRect::new(
        NSPoint::new(slider_x, slider_y),
        NSSize::new(slider_width, slider_height),
    );

    // Get current transparency value
    let current_transparency = TranscriptionWindow::get_transparency();

    let slider = create_slider(
        mtm,
        slider_frame,
        0.3,                  // min (30%)
        1.0,                  // max (100%)
        current_transparency, // current value
        delegate,
        sel!(handleTransparencySlider:),
    );

    // Value label below slider showing percentage
    let value_y: CGFloat = 275.0;
    let value_frame = NSRect::new(
        NSPoint::new(slider_x, value_y),
        NSSize::new(slider_width, value_label_height),
    );
    let percentage = (current_transparency * 100.0).round() as i32;
    let value_label = create_value_label(mtm, value_frame, &format!("{}%", percentage));

    // SAFETY: Adding valid subviews to a valid parent view
    unsafe {
        content_view.addSubview(&label);
        content_view.addSubview(&slider);
        content_view.addSubview(&value_label);
    }

    (slider, value_label)
}
