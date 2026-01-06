//! Header view component with recording type label and hide button

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, msg_send_id};
use objc2_app_kit::{NSColor, NSFont, NSTextField, NSView};
use objc2_foundation::{CGFloat, MainThreadMarker, NSPoint, NSRange, NSRect, NSSize, NSString};
use std::sync::atomic::Ordering;

use crate::transcription_window::delegates::{HoverButton, WindowActionDelegate};
use crate::transcription_window::state::IS_DARK_MODE;

/// Create the header view with recording type label and hide button
pub(in crate::transcription_window) fn create_header(
    mtm: MainThreadMarker,
    window_width: CGFloat,
    window_height: CGFloat,
    header_height: CGFloat,
    delegate: &WindowActionDelegate,
) -> (
    Retained<NSView>,
    Retained<HoverButton>,
    Retained<NSTextField>,
) {
    // Header frame at top of window
    let header_frame = NSRect::new(
        NSPoint::new(0.0, window_height - header_height),
        NSSize::new(window_width, header_height),
    );

    let header_view: Retained<NSView> =
        unsafe { msg_send_id![mtm.alloc::<NSView>(), initWithFrame: header_frame] };

    // Enable layer for the header view (inherits window background)
    unsafe {
        let _: () = msg_send![&header_view, setWantsLayer: true];
        // Autoresizing: width sizable (2) | min Y margin (8) = 10
        // This ensures the header stretches horizontally and stays at the top
        let _: () = msg_send![&header_view, setAutoresizingMask: 10u64];
    }

    // Create recording type label on the left side
    let label_margin: CGFloat = 12.0;
    let label_frame = NSRect::new(
        NSPoint::new(label_margin, (header_height - 16.0) / 2.0),
        NSSize::new(window_width - 60.0, 16.0),
    );

    let recording_type_label: Retained<NSTextField> =
        unsafe { msg_send_id![mtm.alloc::<NSTextField>(), initWithFrame: label_frame] };

    // Load dark mode preference once for use in multiple UI elements
    let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);

    unsafe {
        // Make it a label (non-editable, no border, no background)
        recording_type_label.setEditable(false);
        recording_type_label.setSelectable(false);
        recording_type_label.setBordered(false);
        recording_type_label.setDrawsBackground(false);

        // Set muted text color based on dark mode
        let muted_color = if is_dark {
            NSColor::colorWithRed_green_blue_alpha(0.6, 0.6, 0.6, 1.0)
        } else {
            NSColor::colorWithRed_green_blue_alpha(0.3, 0.3, 0.3, 1.0)
        };
        recording_type_label.setTextColor(Some(&muted_color));

        // Set smaller font
        let font = NSFont::systemFontOfSize(12.0);
        let _: () = msg_send![&recording_type_label, setFont: &*font];

        // Default label text
        recording_type_label.setStringValue(&NSString::from_str("Live Transcription"));

        // Autoresizing: width sizable (2) to stretch with header
        let _: () = msg_send![&recording_type_label, setAutoresizingMask: 2u64];
    }

    // Create hide button with SF Symbol (xmark) on the right side
    // Uses HoverImageButton for tint color change on hover
    let button_size: CGFloat = 28.0; // Increased from 20px for better touch target
    let button_margin: CGFloat = 6.0;
    let button_frame = NSRect::new(
        NSPoint::new(
            window_width - button_size - button_margin,
            (header_height - button_size) / 2.0,
        ),
        NSSize::new(button_size, button_size),
    );

    let hide_button = HoverButton::new(mtm, button_frame);

    unsafe {
        // Use text "X" for close button (more reliable than SF Symbols)
        let title = NSString::from_str("\u{2715}");
        let _: () = msg_send![&hide_button, setTitle: &*title];

        // Style as borderless
        let _: () = msg_send![&hide_button, setBezelStyle: 0u64]; // NSBezelStyleInline
        let _: () = msg_send![&hide_button, setBordered: false];

        // Set font size for the X symbol
        let font = NSFont::systemFontOfSize(16.0);
        let _: () = msg_send![&hide_button, setFont: &*font];

        // Set initial muted gray text color based on dark mode (matches HoverButton's mouseExited color)
        let button_color = if is_dark {
            NSColor::colorWithRed_green_blue_alpha(0.55, 0.55, 0.55, 1.0)
        } else {
            NSColor::colorWithRed_green_blue_alpha(0.35, 0.35, 0.35, 1.0)
        };
        let attr_title: *mut AnyObject = msg_send![&hide_button, attributedTitle];
        if !attr_title.is_null() {
            let mutable_attr: Retained<AnyObject> = msg_send_id![attr_title, mutableCopy];
            let length: usize = msg_send![&mutable_attr, length];
            if length > 0 {
                let range = NSRange::new(0, length);
                let color_key = NSString::from_str("NSColor");
                let _: () = msg_send![&mutable_attr, addAttribute: &*color_key value: &*button_color range: range];
                let _: () = msg_send![&hide_button, setAttributedTitle: &*mutable_attr];
            }
        }

        // Autoresizing: min X margin (1) to stay anchored to right edge
        let _: () = msg_send![&hide_button, setAutoresizingMask: 1u64];

        // Set action with delegate as target
        let _: () = msg_send![&hide_button, setTarget: delegate];
        let _: () = msg_send![&hide_button, setAction: objc2::sel!(handleHide:)];

        // Accessibility: label for VoiceOver
        let accessibility_label = NSString::from_str("Close transcription window");
        let _: () = msg_send![&hide_button, setAccessibilityLabel: &*accessibility_label];
    }

    // Add views to header
    unsafe {
        header_view.addSubview(&recording_type_label);
        header_view.addSubview(&hide_button);
    }

    (header_view, hide_button, recording_type_label)
}
