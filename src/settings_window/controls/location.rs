//! Location control UI elements for the settings window.

use objc2::rc::Retained;
use objc2::sel;
use objc2_app_kit::{NSTextField, NSView};
use objc2_foundation::{CGFloat, MainThreadMarker, NSPoint, NSRect, NSSize};

use super::helpers::{create_path_label, create_section_label, create_small_button};
use crate::settings_window::constants::PADDING;
use crate::settings_window::delegate::SettingsActionDelegate;

/// Configuration for a location section.
struct LocationConfig {
    section_title: &'static str,
    label_y: CGFloat,
    row_y: CGFloat,
    choose_action: objc2::runtime::Sel,
    reset_action: objc2::runtime::Sel,
}

/// Add transcript location control UI elements to the content view.
///
/// Returns the path label so it can be updated when the user changes the location.
pub(crate) fn add_location_controls(
    mtm: MainThreadMarker,
    content_view: &NSView,
    delegate: &SettingsActionDelegate,
    current_path: &str,
) -> Retained<NSTextField> {
    let config = LocationConfig {
        section_title: "Transcript Location",
        label_y: 180.0,
        row_y: 150.0,
        choose_action: sel!(handleChooseLocation:),
        reset_action: sel!(handleResetLocation:),
    };
    add_location_section(mtm, content_view, delegate, current_path, &config)
}

/// Add screenshot location control UI elements to the content view.
///
/// Returns the path label so it can be updated when the user changes the location.
pub(crate) fn add_screenshot_location_controls(
    mtm: MainThreadMarker,
    content_view: &NSView,
    delegate: &SettingsActionDelegate,
    current_path: &str,
) -> Retained<NSTextField> {
    let config = LocationConfig {
        section_title: "Screenshot Location",
        label_y: 105.0,
        row_y: 75.0,
        choose_action: sel!(handleChooseScreenshotLocation:),
        reset_action: sel!(handleResetScreenshotLocation:),
    };
    add_location_section(mtm, content_view, delegate, current_path, &config)
}

/// Generic location section builder with horizontal layout.
/// Layout: Label on top, then [path | Choose | Reset to Default] on same row.
fn add_location_section(
    mtm: MainThreadMarker,
    content_view: &NSView,
    delegate: &SettingsActionDelegate,
    current_path: &str,
    config: &LocationConfig,
) -> Retained<NSTextField> {
    // Get content view width for layout calculations
    let content_width = content_view.frame().size.width;

    let label_height: CGFloat = 20.0;
    let row_height: CGFloat = 24.0;
    let choose_button_width: CGFloat = 75.0;
    let reset_button_width: CGFloat = 110.0;
    let button_gap: CGFloat = 8.0;

    // Calculate path label width (remaining space after buttons)
    let buttons_total_width = choose_button_width + reset_button_width + button_gap;
    let path_width = content_width - PADDING * 2.0 - buttons_total_width - button_gap;

    // Section label
    let label_frame = NSRect::new(
        NSPoint::new(PADDING, config.label_y),
        NSSize::new(content_width - PADDING * 2.0, label_height),
    );
    let label = create_section_label(mtm, label_frame, config.section_title);

    // Path display label (left side of row)
    let path_frame = NSRect::new(
        NSPoint::new(PADDING, config.row_y),
        NSSize::new(path_width, row_height),
    );
    let path_label = create_path_label(mtm, path_frame, current_path);

    // Choose button (after path)
    let choose_x = PADDING + path_width + button_gap;
    let choose_button_frame = NSRect::new(
        NSPoint::new(choose_x, config.row_y),
        NSSize::new(choose_button_width, row_height),
    );
    let choose_button = create_small_button(
        mtm,
        choose_button_frame,
        "Choose...",
        delegate,
        config.choose_action,
    );

    // Reset to Default button (after Choose)
    let reset_x = choose_x + choose_button_width + button_gap;
    let reset_button_frame = NSRect::new(
        NSPoint::new(reset_x, config.row_y),
        NSSize::new(reset_button_width, row_height),
    );
    let reset_button = create_small_button(
        mtm,
        reset_button_frame,
        "Reset to Default",
        delegate,
        config.reset_action,
    );

    // SAFETY: Adding valid subviews to a valid parent view
    unsafe {
        content_view.addSubview(&label);
        content_view.addSubview(&path_label);
        content_view.addSubview(&choose_button);
        content_view.addSubview(&reset_button);
    }

    path_label
}
