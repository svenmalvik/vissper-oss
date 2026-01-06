//! Objective-C delegate classes for window event handling

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{declare_class, msg_send, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{NSButton, NSColor, NSView};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSRange, NSRect, NSString};

use crate::transcription_window::TranscriptionWindow;

// Delegate class for handling button actions
declare_class!(
    pub struct WindowActionDelegate;

    unsafe impl ClassType for WindowActionDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "VissperWindowActionDelegate";
    }

    impl DeclaredClass for WindowActionDelegate {}

    unsafe impl WindowActionDelegate {
        #[method(handleHide:)]
        fn handle_hide(&self, _sender: *mut NSObject) {
            TranscriptionWindow::handle_hide_action();
        }

        #[method(handleLessTransparent:)]
        fn handle_less_transparent(&self, _sender: *mut NSObject) {
            TranscriptionWindow::adjust_transparency(-0.1);
        }

        #[method(handleMoreTransparent:)]
        fn handle_more_transparent(&self, _sender: *mut NSObject) {
            TranscriptionWindow::adjust_transparency(0.1);
        }

        #[method(handleSaveFile:)]
        fn handle_save_file(&self, _sender: *mut NSObject) {
            TranscriptionWindow::handle_save_file_action();
        }

        #[method(handleTabChange:)]
        fn handle_tab_change(&self, sender: *mut NSObject) {
            // Get selected segment index from the segmented control
            let selected_index: isize = unsafe { msg_send![sender, selectedSegment] };
            TranscriptionWindow::handle_tab_change_action(selected_index);
        }
    }

    unsafe impl NSObjectProtocol for WindowActionDelegate {}
);

impl WindowActionDelegate {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let alloc = mtm.alloc::<Self>();
        unsafe { msg_send_id![alloc, init] }
    }
}

// Custom content view class for tracking mouse enter/exit
declare_class!(
    pub struct TrackingContentView;

    unsafe impl ClassType for TrackingContentView {
        type Super = NSView;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "VissperTrackingContentView";
    }

    impl DeclaredClass for TrackingContentView {}

    unsafe impl TrackingContentView {
        #[method(mouseEntered:)]
        fn mouse_entered(&self, _event: *mut AnyObject) {
            // No-op: hover controls removed
        }

        #[method(mouseExited:)]
        fn mouse_exited(&self, _event: *mut AnyObject) {
            // No-op: hover controls removed
        }

        #[method(updateTrackingAreas)]
        fn update_tracking_areas(&self) {
            unsafe {
                // Call super
                let _: () = msg_send![super(self), updateTrackingAreas];

                // Remove existing tracking areas
                let tracking_areas: *mut AnyObject = msg_send![self, trackingAreas];
                if !tracking_areas.is_null() {
                    let count: usize = msg_send![tracking_areas, count];
                    for i in 0..count {
                        let area: *mut AnyObject = msg_send![tracking_areas, objectAtIndex: i];
                        let _: () = msg_send![self, removeTrackingArea: area];
                    }
                }

                // Add new tracking area for entire view bounds
                let bounds: NSRect = msg_send![self, bounds];

                // NSTrackingMouseEnteredAndExited | NSTrackingActiveAlways
                let options: usize = 0x01 | 0x80;

                let tracking_area_class = objc2::runtime::AnyClass::get("NSTrackingArea")
                    .expect("NSTrackingArea class must exist on macOS");
                let tracking_area: *mut AnyObject = msg_send![tracking_area_class, alloc];
                let tracking_area: *mut AnyObject = msg_send![
                    tracking_area,
                    initWithRect: bounds
                    options: options
                    owner: self
                    userInfo: std::ptr::null::<AnyObject>()
                ];

                if !tracking_area.is_null() {
                    let _: () = msg_send![self, addTrackingArea: tracking_area];
                }
            }
        }
    }

    unsafe impl NSObjectProtocol for TrackingContentView {}
);

impl TrackingContentView {
    pub fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let alloc = mtm.alloc::<Self>();
        unsafe { msg_send_id![alloc, initWithFrame: frame] }
    }
}

// Custom button class that lightens text on hover
declare_class!(
    pub struct HoverButton;

    unsafe impl ClassType for HoverButton {
        type Super = NSButton;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "VissperHoverButton";
    }

    impl DeclaredClass for HoverButton {}

    unsafe impl HoverButton {
        #[method(mouseEntered:)]
        fn mouse_entered(&self, _event: *mut AnyObject) {
            // Lighter text color on hover
            unsafe {
                let lighter_color = NSColor::colorWithRed_green_blue_alpha(0.75, 0.75, 0.75, 1.0);
                Self::set_title_color(self, &lighter_color);
            }
        }

        #[method(mouseExited:)]
        fn mouse_exited(&self, _event: *mut AnyObject) {
            // Original muted color
            unsafe {
                let muted_color = NSColor::colorWithRed_green_blue_alpha(0.55, 0.55, 0.55, 1.0);
                Self::set_title_color(self, &muted_color);
            }
        }

        #[method(updateTrackingAreas)]
        fn update_tracking_areas(&self) {
            unsafe {
                // Call super
                let _: () = msg_send![super(self), updateTrackingAreas];

                // Remove existing tracking areas
                let tracking_areas: *mut AnyObject = msg_send![self, trackingAreas];
                if !tracking_areas.is_null() {
                    let count: usize = msg_send![tracking_areas, count];
                    for i in 0..count {
                        let area: *mut AnyObject = msg_send![tracking_areas, objectAtIndex: i];
                        let _: () = msg_send![self, removeTrackingArea: area];
                    }
                }

                // Add tracking area for button bounds
                let bounds: NSRect = msg_send![self, bounds];

                // NSTrackingMouseEnteredAndExited | NSTrackingActiveAlways
                let options: usize = 0x01 | 0x80;

                let tracking_area_class = objc2::runtime::AnyClass::get("NSTrackingArea")
                    .expect("NSTrackingArea class must exist on macOS");
                let tracking_area: *mut AnyObject = msg_send![tracking_area_class, alloc];
                let tracking_area: *mut AnyObject = msg_send![
                    tracking_area,
                    initWithRect: bounds
                    options: options
                    owner: self
                    userInfo: std::ptr::null::<AnyObject>()
                ];

                if !tracking_area.is_null() {
                    let _: () = msg_send![self, addTrackingArea: tracking_area];
                }
            }
        }
    }

    unsafe impl NSObjectProtocol for HoverButton {}
);

impl HoverButton {
    pub fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let alloc = mtm.alloc::<Self>();
        unsafe { msg_send_id![alloc, initWithFrame: frame] }
    }

    /// Helper to set the button's title color via attributed string
    unsafe fn set_title_color(&self, color: &NSColor) {
        let attr_title: *mut AnyObject = msg_send![self, attributedTitle];
        if attr_title.is_null() {
            return;
        }
        let mutable_attr: Retained<AnyObject> = msg_send_id![attr_title, mutableCopy];
        let length: usize = msg_send![&mutable_attr, length];
        if length > 0 {
            let range = NSRange::new(0, length);
            let color_key = NSString::from_str("NSColor");
            let _: () =
                msg_send![&mutable_attr, addAttribute: &*color_key value: color range: range];
            let _: () = msg_send![self, setAttributedTitle: &*mutable_attr];
        }
    }
}
