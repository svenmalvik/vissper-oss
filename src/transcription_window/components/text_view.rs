//! Scrollable text view component for displaying transcription content

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, msg_send_id, ClassType};
use objc2_app_kit::{NSColor, NSFont, NSScrollView, NSTextView};
use objc2_foundation::{
    CGFloat, MainThreadMarker, NSMutableAttributedString, NSPoint, NSRange, NSRect, NSSize,
    NSString,
};
use std::sync::atomic::Ordering;

use crate::transcription_window::state::IS_DARK_MODE;

/// Create a scrollable text view for displaying transcription text
pub(in crate::transcription_window) fn create_scrollable_text_view(
    mtm: MainThreadMarker,
    width: CGFloat,
    content_height: CGFloat,
    footer_height: CGFloat,
    padding: CGFloat,
    initial_text: &str,
    visible: bool,
) -> (Retained<NSScrollView>, Retained<NSTextView>) {
    // Position scroll view between tab control and footer
    let scroll_frame = NSRect::new(
        NSPoint::new(padding, footer_height),
        NSSize::new(width - (padding * 2.0), content_height - padding),
    );

    // Create scroll view
    let scroll_view: Retained<NSScrollView> =
        unsafe { msg_send_id![mtm.alloc::<NSScrollView>(), initWithFrame: scroll_frame] };

    unsafe {
        // Configure scroll view
        scroll_view.setHasVerticalScroller(true);
        scroll_view.setHasHorizontalScroller(false);
        let _: () = msg_send![&scroll_view, setAutohidesScrollers: true];

        // Make scroll view background transparent
        scroll_view.setDrawsBackground(false);
        let _: () = msg_send![&scroll_view, setBorderType: 0u64]; // NSNoBorder

        // Resize with window (width sizable | height sizable)
        let _: () = msg_send![&scroll_view, setAutoresizingMask: 18u64];

        // Set initial visibility
        let _: () = msg_send![&scroll_view, setHidden: !visible];
    }

    // Create text view with the same frame (will be resized by scroll view)
    let text_frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(scroll_frame.size.width, scroll_frame.size.height),
    );

    let text_view: Retained<NSTextView> =
        unsafe { msg_send_id![mtm.alloc::<NSTextView>(), initWithFrame: text_frame] };

    unsafe {
        // Make text view non-editable but selectable
        text_view.setEditable(false);
        text_view.setSelectable(true);

        // Transparent background
        text_view.setDrawsBackground(false);

        // Text color based on saved dark mode preference
        let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);
        let text_color = if is_dark {
            NSColor::whiteColor()
        } else {
            NSColor::blackColor()
        };
        text_view.setTextColor(Some(&text_color));

        // Set font - monospaced system font, 14pt
        let font: Retained<NSFont> = msg_send_id![
            NSFont::class(),
            monospacedSystemFontOfSize: 14.0,
            weight: 0.0
        ];
        text_view.setFont(Some(&font));

        // Configure text container for word wrapping
        let text_container: *mut AnyObject = msg_send![&text_view, textContainer];
        if !text_container.is_null() {
            let _: () = msg_send![text_container, setWidthTracksTextView: true];
            let container_size = NSSize::new(scroll_frame.size.width - 10.0, CGFloat::MAX);
            let _: () = msg_send![text_container, setContainerSize: container_size];
        }

        // Make text view resize with scroll view
        let _: () = msg_send![&text_view, setMinSize: NSSize::new(0.0, scroll_frame.size.height)];
        let _: () = msg_send![&text_view, setMaxSize: NSSize::new(CGFloat::MAX, CGFloat::MAX)];
        let _: () = msg_send![&text_view, setVerticallyResizable: true];
        let _: () = msg_send![&text_view, setHorizontallyResizable: false];

        // Set initial placeholder text with padding at the end (as attributed string)
        let padded_text = format!("{}\n\n\n\n\n\n", initial_text);
        let initial_str = NSString::from_str(&padded_text);
        let attr_string: Retained<NSMutableAttributedString> =
            msg_send_id![NSMutableAttributedString::alloc(), initWithString: &*initial_str];

        // Apply foreground color based on dark mode
        let length: usize = msg_send![&attr_string, length];
        let full_range = NSRange::new(0, length);
        let color_attr_name = NSString::from_str("NSColor");
        let _: () = msg_send![
            &attr_string,
            addAttribute: &*color_attr_name,
            value: &*text_color,
            range: full_range
        ];

        // Set attributed string via text storage
        let text_storage: *mut AnyObject = msg_send![&text_view, textStorage];
        if !text_storage.is_null() {
            let _: () = msg_send![text_storage, setAttributedString: &*attr_string];
        }
    }

    // Set the text view as the document view of the scroll view
    unsafe {
        scroll_view.setDocumentView(Some(&text_view));
    }

    (scroll_view, text_view)
}
