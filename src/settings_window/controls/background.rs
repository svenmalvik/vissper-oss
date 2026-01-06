//! Background color control UI elements for the settings window.

use objc2::rc::Retained;
use objc2::sel;
use objc2_app_kit::{NSSegmentedControl, NSView};
use objc2_foundation::{CGFloat, MainThreadMarker, NSPoint, NSRect, NSSize};

use super::helpers::{create_section_label, create_segmented_control};
use crate::settings_window::constants::PADDING;
use crate::settings_window::delegate::SettingsActionDelegate;
use crate::transcription_window::TranscriptionWindow;

/// Add background color control UI elements to the content view.
///
/// Returns the segmented control so it can be updated.
pub(crate) fn add_background_controls(
    mtm: MainThreadMarker,
    content_view: &NSView,
    delegate: &SettingsActionDelegate,
) -> Retained<NSSegmentedControl> {
    // Get content view width for layout calculations
    let content_width = content_view.frame().size.width;

    let label_height: CGFloat = 20.0;
    let control_width: CGFloat = 160.0;
    let control_height: CGFloat = 24.0;

    // Section label below transparency separator
    let label_y: CGFloat = 245.0;
    let label_frame = NSRect::new(
        NSPoint::new(PADDING, label_y),
        NSSize::new(content_width - PADDING * 2.0, label_height),
    );
    let label = create_section_label(mtm, label_frame, "Background");

    // Segmented control centered below label
    let control_y: CGFloat = 210.0;
    let control_x = (content_width - control_width) / 2.0;
    let control_frame = NSRect::new(
        NSPoint::new(control_x, control_y),
        NSSize::new(control_width, control_height),
    );

    // Determine initial selection based on current mode
    let is_dark = TranscriptionWindow::is_dark_mode();
    let selected_segment = if is_dark { 0 } else { 1 }; // 0 = Dark, 1 = Light

    let segmented_control = create_segmented_control(
        mtm,
        control_frame,
        &["Dark", "Light"],
        selected_segment,
        delegate,
        sel!(handleBackgroundSegment:),
    );

    // SAFETY: Adding valid subviews to a valid parent view
    unsafe {
        content_view.addSubview(&label);
        content_view.addSubview(&segmented_control);
    }

    segmented_control
}
