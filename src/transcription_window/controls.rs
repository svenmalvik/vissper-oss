//! UI control creation functions for recording indicator and saved file button

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, msg_send_id, sel, ClassType};
use objc2_app_kit::{NSColor, NSFont, NSImage, NSImageView, NSTextAlignment, NSTextField, NSView};
use objc2_foundation::{CGFloat, MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use std::sync::atomic::Ordering;
use tracing::warn;

use super::delegates::{HoverButton, WindowActionDelegate};
use super::objc_utils;
use super::state::IS_DARK_MODE;

/// Create the recording indicator (SF Symbol + "Recording" text) at center bottom
pub(super) fn create_recording_indicator(
    mtm: MainThreadMarker,
    window_width: CGFloat,
) -> (Retained<NSView>, Retained<NSTextField>) {
    let icon_size: CGFloat = 14.0;
    let label_width: CGFloat = 80.0;
    let total_width: CGFloat = icon_size + 6.0 + label_width;
    let indicator_y: CGFloat = 15.0;

    // SF Symbol recording indicator (record.circle.fill)
    let icon_frame = NSRect::new(
        NSPoint::new((window_width - total_width) / 2.0, indicator_y),
        NSSize::new(icon_size, icon_size),
    );

    let icon_view = unsafe { NSImageView::initWithFrame(mtm.alloc(), icon_frame) };

    unsafe {
        // Create SF Symbol image for recording indicator
        let symbol_name = NSString::from_str("record.circle.fill");
        let accessibility_desc = NSString::from_str("Recording");

        // Use NSImage imageWithSystemSymbolName:accessibilityDescription:
        let image: Option<Retained<NSImage>> = msg_send_id![
            NSImage::class(),
            imageWithSystemSymbolName: &*symbol_name,
            accessibilityDescription: &*accessibility_desc
        ];

        if let Some(image) = image {
            // Configure symbol with appropriate size using graceful fallback
            // Note: weight is CGFloat (NSFontWeightRegular = 0.0), scale is NSInteger
            if let Some(config_class) = objc_utils::get_class_or_warn("NSImageSymbolConfiguration")
            {
                let config: *mut AnyObject = msg_send![
                    config_class,
                    configurationWithPointSize: 12.0f64,
                    weight: 0.0f64, // NSFontWeightRegular (CGFloat)
                    scale: 1isize  // NSImageSymbolScaleMedium
                ];

                if !config.is_null() {
                    let configured_image: Option<Retained<NSImage>> =
                        msg_send_id![&image, imageWithSymbolConfiguration: config];
                    if let Some(configured_image) = configured_image {
                        icon_view.setImage(Some(&configured_image));
                    } else {
                        icon_view.setImage(Some(&image));
                    }
                } else {
                    icon_view.setImage(Some(&image));
                }
            } else {
                // Fallback: use image without symbol configuration
                icon_view.setImage(Some(&image));
            }

            // Tint the symbol red using contentTintColor
            let red_color = NSColor::colorWithRed_green_blue_alpha(0.9, 0.2, 0.2, 1.0);
            let _: () = msg_send![&icon_view, setContentTintColor: &*red_color];
        }

        // Initially hidden
        let _: () = msg_send![&icon_view, setHidden: true];

        // Autoresizing: min X margin (1) | max X margin (4) = 5 (center horizontally)
        // max Y margin (32) keeps it at the bottom
        let _: () = msg_send![&icon_view, setAutoresizingMask: 37u64];

        // Accessibility: label for VoiceOver
        let accessibility_label = NSString::from_str("Recording in progress");
        let _: () = msg_send![&icon_view, setAccessibilityLabel: &*accessibility_label];
    }

    // "Recording" label
    let label_frame = NSRect::new(
        NSPoint::new(
            (window_width - total_width) / 2.0 + icon_size + 6.0,
            indicator_y - 1.0,
        ),
        NSSize::new(label_width, 16.0),
    );

    let label: Retained<NSTextField> =
        unsafe { msg_send_id![mtm.alloc::<NSTextField>(), initWithFrame: label_frame] };

    unsafe {
        label.setEditable(false);
        label.setSelectable(false);
        label.setBordered(false);
        label.setDrawsBackground(false);
        label.setStringValue(&NSString::from_str("Recording"));

        // Red text to match icon
        let red_color = NSColor::colorWithRed_green_blue_alpha(0.9, 0.3, 0.3, 1.0);
        label.setTextColor(Some(&red_color));

        // Font
        let font = NSFont::systemFontOfSize(12.0);
        label.setFont(Some(&font));

        label.setAlignment(NSTextAlignment::Left);

        // Initially hidden
        let _: () = msg_send![&label, setHidden: true];

        // Autoresizing: min X margin (1) | max X margin (4) = 5 (center horizontally)
        // max Y margin (32) keeps it at the bottom
        let _: () = msg_send![&label, setAutoresizingMask: 37u64];
    }

    // Cast NSImageView to NSView with graceful fallback
    let view_ptr: *const NSImageView = &*icon_view;
    let view_ptr = view_ptr as *mut NSView;
    let icon_as_view = match unsafe { Retained::retain(view_ptr) } {
        Some(view) => view,
        None => {
            warn!("Failed to retain NSImageView as NSView - using empty view");
            // Create a minimal fallback view
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(icon_size, icon_size));
            unsafe { msg_send_id![mtm.alloc::<NSView>(), initWithFrame: frame] }
        }
    };

    (icon_as_view, label)
}

/// Create the "Save" button at center bottom with SF Symbol icon
/// This button is shown after recording stops, allowing users to save the transcript
pub(super) fn create_save_button(
    mtm: MainThreadMarker,
    window_width: CGFloat,
    delegate: &WindowActionDelegate,
) -> Retained<HoverButton> {
    let button_width: CGFloat = 80.0; // Smaller width since we removed verbose text
    let button_height: CGFloat = 24.0;
    let button_y: CGFloat = 13.0;

    let button_frame = NSRect::new(
        NSPoint::new((window_width - button_width) / 2.0, button_y),
        NSSize::new(button_width, button_height),
    );

    let button = HoverButton::new(mtm, button_frame);

    unsafe {
        // Create SF Symbol for save action (square.and.arrow.down)
        let symbol_name = NSString::from_str("square.and.arrow.down");
        let accessibility_desc = NSString::from_str("Save transcript");

        let image: Option<Retained<NSImage>> = msg_send_id![
            NSImage::class(),
            imageWithSystemSymbolName: &*symbol_name,
            accessibilityDescription: &*accessibility_desc
        ];

        if let Some(image) = image {
            // Set image on button
            let _: () = msg_send![&button, setImage: &*image];
            let _: () = msg_send![&button, setImagePosition: 2usize]; // NSImageLeft
        }

        // Set button title
        let title = NSString::from_str("Save");
        let _: () = msg_send![&button, setTitle: &*title];

        // Style as borderless/plain
        let _: () = msg_send![&button, setBezelStyle: 0u64]; // NSBezelStyleInline
        let _: () = msg_send![&button, setBordered: false];

        // Muted gray text color and tint based on dark mode
        let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);
        let muted_color = if is_dark {
            NSColor::colorWithRed_green_blue_alpha(0.55, 0.55, 0.55, 1.0)
        } else {
            NSColor::colorWithRed_green_blue_alpha(0.35, 0.35, 0.35, 1.0)
        };
        let _: () = msg_send![&button, setContentTintColor: &*muted_color];

        let attr_title: *mut AnyObject = msg_send![&button, attributedTitle];
        if !attr_title.is_null() {
            let mutable_attr: Retained<AnyObject> = msg_send_id![attr_title, mutableCopy];
            let length: usize = msg_send![&mutable_attr, length];
            if length > 0 {
                let range = objc2_foundation::NSRange::new(0, length);
                let color_key = NSString::from_str("NSColor");
                let _: () = msg_send![&mutable_attr, addAttribute: &*color_key value: &*muted_color range: range];
                let _: () = msg_send![&button, setAttributedTitle: &*mutable_attr];
            }
        }

        // Font - slightly smaller, system font
        let font = NSFont::systemFontOfSize(12.0);
        let _: () = msg_send![&button, setFont: &*font];

        // Initially hidden
        let _: () = msg_send![&button, setHidden: true];

        // Autoresizing: min X margin (1) | max X margin (4) = 5 (center horizontally)
        // max Y margin (32) keeps it at the bottom
        let _: () = msg_send![&button, setAutoresizingMask: 37u64];

        // Set action with delegate as target
        let _: () = msg_send![&button, setTarget: delegate];
        let _: () = msg_send![&button, setAction: sel!(handleSaveFile:)];

        // Accessibility: label for VoiceOver
        let accessibility_label = NSString::from_str("Save transcript to file");
        let _: () = msg_send![&button, setAccessibilityLabel: &*accessibility_label];
    }

    button
}
