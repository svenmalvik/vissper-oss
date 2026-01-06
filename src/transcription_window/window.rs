//! Window creation and orchestration

use objc2::msg_send;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSBackingStoreType, NSColor, NSScreen, NSWindow, NSWindowStyleMask};
use objc2_foundation::{CGFloat, MainThreadMarker, NSPoint, NSRect, NSSize};
use std::sync::atomic::Ordering;
use tracing::info;

use super::components::{create_header, create_scrollable_text_view, create_tab_control};
use super::controls::{create_recording_indicator, create_save_button};
use super::delegates::{TrackingContentView, WindowActionDelegate};
use super::state::{
    TabContent, TabType, TranscriptionWindowInner, CURRENT_TRANSPARENCY, IS_DARK_MODE,
};

/// Create the transparent window with all UI elements
pub(super) fn create_window(mtm: MainThreadMarker) -> TranscriptionWindowInner {
    // Create delegate for button actions
    let delegate = WindowActionDelegate::new(mtm);

    // Layout constants
    let header_height: CGFloat = 30.0;
    let tab_height: CGFloat = 24.0; // Height for segmented control
    let footer_height: CGFloat = 50.0; // Space for recording indicator and hover controls
    let padding: CGFloat = 16.0;

    // Get main screen dimensions for positioning
    let main_screen = NSScreen::mainScreen(mtm);
    let screen_frame = match main_screen {
        Some(screen) => screen.frame(),
        None => NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1920.0, 1080.0)),
    };

    // Window size - 30% of screen width and 30% of screen height
    let window_width: CGFloat = screen_frame.size.width * 0.30;
    let window_height: CGFloat = screen_frame.size.height * 0.30;

    // Position at vertically centered on the right edge with a small margin
    // Must add screen_frame.origin to handle multi-monitor setups correctly
    let padding_right: CGFloat = 20.0;
    let origin_x = screen_frame.origin.x + screen_frame.size.width - window_width - padding_right;
    let origin_y = screen_frame.origin.y + (screen_frame.size.height - window_height) / 2.0;

    let frame = NSRect::new(
        NSPoint::new(origin_x, origin_y),
        NSSize::new(window_width, window_height),
    );

    // Create borderless window
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            frame,
            NSWindowStyleMask::Borderless | NSWindowStyleMask::Resizable,
            NSBackingStoreType::NSBackingStoreBuffered,
            false,
        )
    };

    // Mark as released when closed = false for proper memory management
    unsafe { window.setReleasedWhenClosed(false) };

    // Make window transparent so background color with alpha is visible
    window.setOpaque(false);

    // Set initial semi-transparent background color from saved preferences
    // This will be adjustable via the transparency slider
    unsafe {
        let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);
        let transparency = CURRENT_TRANSPARENCY.load(Ordering::SeqCst) as f64 / 100.0;
        let base = if is_dark { 0.1 } else { 0.95 };
        let bg_color = NSColor::colorWithRed_green_blue_alpha(base, base, base, transparency);
        window.setBackgroundColor(Some(&bg_color));
    }

    // Set window level to float above other windows (NSFloatingWindowLevel = 3)
    window.setLevel(3);

    // Add shadow for better visual distinction
    unsafe {
        let _: () = msg_send![&window, setHasShadow: true];
    }

    // Configure window behavior for overlay
    unsafe {
        // Don't hide when app deactivates (so it stays visible when working in other apps)
        let _: () = msg_send![&window, setHidesOnDeactivate: false];

        // Allow mouse events for interaction
        let _: () = msg_send![&window, setIgnoresMouseEvents: false];

        // Make window movable by dragging anywhere in the window
        let _: () = msg_send![&window, setMovableByWindowBackground: true];
    }

    // Create content view frame
    let content_frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(window_width, window_height),
    );

    // Create custom tracking content view as the content view
    // (for mouse enter/exit tracking)
    // Window backgroundColor provides the semi-transparent background
    let tracking_content_view = TrackingContentView::new(mtm, content_frame);

    unsafe {
        let _: () = msg_send![&tracking_content_view, setAutoresizesSubviews: true];
        // Autoresizing: width sizable (2) | height sizable (16) = 18
        let _: () = msg_send![&tracking_content_view, setAutoresizingMask: 18u64];

        // Enable layer backing for rounded corners
        let _: () = msg_send![&tracking_content_view, setWantsLayer: true];

        // Add rounded corners to the content view
        let layer: *mut AnyObject = msg_send![&tracking_content_view, layer];
        if !layer.is_null() {
            let _: () = msg_send![layer, setCornerRadius: 12.0f64];
            let _: () = msg_send![layer, setMasksToBounds: true];
        }
    }

    window.setContentView(Some(&tracking_content_view));

    // Create header view with recording type label and hide button
    let (header_view, hide_button, recording_type_label) =
        create_header(mtm, window_width, window_height, header_height, &delegate);

    // Create tab control (segmented control)
    // NSSegmentedControl is a core macOS class - if it fails, the UI is fundamentally broken
    let segmented_control = create_tab_control(
        mtm,
        window_width,
        window_height,
        header_height,
        tab_height,
        &delegate,
    )
    .expect("NSSegmentedControl is a core macOS class and must be available");

    // Calculate content height for text views (below header and tab control, above footer)
    let content_height = window_height - header_height - tab_height - footer_height - 8.0; // 8.0 for spacing

    // Create three text views for each tab
    // Tab 1: Live transcription (visible by default)
    let (live_scroll_view, live_text_view) = create_scrollable_text_view(
        mtm,
        window_width,
        content_height,
        footer_height,
        padding,
        "Listening...",
        true,
    );

    // Tab 2: Polished transcript (hidden by default)
    let (polished_scroll_view, polished_text_view) = create_scrollable_text_view(
        mtm,
        window_width,
        content_height,
        footer_height,
        padding,
        "Click to generate polished transcript...",
        false,
    );

    // Tab 3: Meeting notes (hidden by default)
    let (meeting_scroll_view, meeting_text_view) = create_scrollable_text_view(
        mtm,
        window_width,
        content_height,
        footer_height,
        padding,
        "Click to generate meeting notes...",
        false,
    );

    // Create recording indicator (center bottom)
    let (recording_indicator, recording_label) = create_recording_indicator(mtm, window_width);

    // Create save button (center bottom, shown after recording to allow manual save)
    let save_button = create_save_button(mtm, window_width, &delegate);

    // Add all views to the tracking content view
    unsafe {
        tracking_content_view.addSubview(&header_view);
        tracking_content_view.addSubview(&segmented_control);
        tracking_content_view.addSubview(&live_scroll_view);
        tracking_content_view.addSubview(&polished_scroll_view);
        tracking_content_view.addSubview(&meeting_scroll_view);
        tracking_content_view.addSubview(&recording_indicator);
        tracking_content_view.addSubview(&recording_label);
        tracking_content_view.addSubview(&save_button);
    }

    // Show the window - use makeKeyAndOrderFront to ensure visibility
    window.makeKeyAndOrderFront(None);

    info!("Transcription window created and shown");

    TranscriptionWindowInner {
        window,
        segmented_control,
        active_tab: TabType::Live,
        tab_content: TabContent::default(),
        live_scroll_view,
        live_text_view,
        polished_scroll_view,
        polished_text_view,
        meeting_scroll_view,
        meeting_text_view,
        header_view,
        hide_button,
        recording_type_label,
        recording_indicator,
        recording_label,
        save_button,
        delegate,
    }
}
