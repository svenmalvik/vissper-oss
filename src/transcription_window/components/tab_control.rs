//! Tab segmented control component for switching between transcription views

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, sel};
use objc2_app_kit::NSView;
use objc2_foundation::{CGFloat, MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use tracing::warn;

use crate::transcription_window::delegates::WindowActionDelegate;
use crate::transcription_window::objc_utils;

/// Create the segmented control for tab switching
///
/// Uses raw Objective-C calls since NSSegmentedControl is not directly exported by objc2_app_kit.
/// Returns `None` if the NSSegmentedControl class is not available (graceful degradation).
pub(in crate::transcription_window) fn create_tab_control(
    _mtm: MainThreadMarker,
    window_width: CGFloat,
    window_height: CGFloat,
    header_height: CGFloat,
    tab_height: CGFloat,
    delegate: &WindowActionDelegate,
) -> Option<Retained<NSView>> {
    let padding: CGFloat = 16.0;
    let control_width: CGFloat = window_width - (padding * 2.0);

    // Position below header
    let control_frame = NSRect::new(
        NSPoint::new(padding, window_height - header_height - tab_height - 4.0),
        NSSize::new(control_width, tab_height),
    );

    // Get NSSegmentedControl class with graceful fallback
    let segmented_class = objc_utils::get_class_or_warn("NSSegmentedControl")?;

    let segmented_control: Retained<NSView> = unsafe {
        let obj: *mut AnyObject = msg_send![segmented_class, alloc];
        let obj: *mut AnyObject = msg_send![obj, initWithFrame: control_frame];

        // Gracefully handle retain failure
        match objc_utils::retain_as_view(obj) {
            Some(view) => view,
            None => {
                warn!("Failed to create NSSegmentedControl - tab switching will be unavailable");
                return None;
            }
        }
    };

    unsafe {
        // Set segment count
        let _: () = msg_send![&segmented_control, setSegmentCount: 3isize];

        // Set segment labels
        let live_label = NSString::from_str("Live");
        let polished_label = NSString::from_str("Polished");
        let meeting_label = NSString::from_str("Meeting Notes");
        let _: () = msg_send![&segmented_control, setLabel: &*live_label forSegment: 0isize];
        let _: () = msg_send![&segmented_control, setLabel: &*polished_label forSegment: 1isize];
        let _: () = msg_send![&segmented_control, setLabel: &*meeting_label forSegment: 2isize];

        // Set segment widths (0.0 = auto-size based on content)
        let _: () = msg_send![&segmented_control, setWidth: 0.0f64 forSegment: 0isize];
        let _: () = msg_send![&segmented_control, setWidth: 0.0f64 forSegment: 1isize];
        let _: () = msg_send![&segmented_control, setWidth: 0.0f64 forSegment: 2isize];

        // Style as capsule/rounded (NSSegmentStyleCapsule = 5)
        let _: () = msg_send![&segmented_control, setSegmentStyle: 5isize];

        // Select the first segment (Live) by default
        let _: () = msg_send![&segmented_control, setSelectedSegment: 0isize];

        // Set action for tab changes
        let _: () = msg_send![&segmented_control, setTarget: delegate];
        let _: () = msg_send![&segmented_control, setAction: sel!(handleTabChange:)];

        // Autoresizing: width sizable (2) | min Y margin (8) = 10
        let _: () = msg_send![&segmented_control, setAutoresizingMask: 10u64];

        // Accessibility: label for VoiceOver
        let accessibility_label = NSString::from_str("Transcription view selector");
        let _: () = msg_send![&segmented_control, setAccessibilityLabel: &*accessibility_label];
    }

    Some(segmented_control)
}
