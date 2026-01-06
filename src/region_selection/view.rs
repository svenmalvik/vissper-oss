//! Custom NSView for region selection mouse handling and drawing.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{declare_class, msg_send, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{NSColor, NSView};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSPoint, NSRect};

use super::RegionSelection;

// Custom NSView for handling mouse events and drawing selection rectangle
declare_class!(
    pub struct RegionSelectionView;

    unsafe impl ClassType for RegionSelectionView {
        type Super = NSView;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "VissperRegionSelectionView";
    }

    impl DeclaredClass for RegionSelectionView {}

    unsafe impl RegionSelectionView {
        #[method(mouseDown:)]
        fn mouse_down(&self, event: *mut AnyObject) {
            if event.is_null() {
                return;
            }

            // Get click location in window coordinates
            let location: NSPoint = unsafe { msg_send![event, locationInWindow] };

            // Convert to screen coordinates
            let window: *mut AnyObject = unsafe { msg_send![self, window] };
            if window.is_null() {
                return;
            }
            let screen_location: NSPoint =
                unsafe { msg_send![window, convertPointToScreen: location] };

            RegionSelection::set_selection_origin(screen_location);
        }

        #[method(mouseDragged:)]
        fn mouse_dragged(&self, event: *mut AnyObject) {
            if event.is_null() {
                return;
            }

            // Get drag location in window coordinates
            let location: NSPoint = unsafe { msg_send![event, locationInWindow] };

            // Convert to screen coordinates
            let window: *mut AnyObject = unsafe { msg_send![self, window] };
            if window.is_null() {
                return;
            }
            let screen_location: NSPoint =
                unsafe { msg_send![window, convertPointToScreen: location] };

            RegionSelection::update_selection(screen_location);
            RegionSelection::request_redraw();
        }

        #[method(mouseUp:)]
        fn mouse_up(&self, _event: *mut AnyObject) {
            RegionSelection::complete_selection();
        }

        #[method(drawRect:)]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            // Get current selection rectangle
            let selection_rect = RegionSelection::get_current_rect();

            if let Some(rect) = selection_rect {
                // Convert screen coordinates to view coordinates
                let window: *mut AnyObject = unsafe { msg_send![self, window] };
                if window.is_null() {
                    return;
                }

                let view_rect: NSRect =
                    unsafe { msg_send![window, convertRectFromScreen: rect] };

                unsafe {
                    // Draw selection rectangle fill (light blue)
                    let fill_color = NSColor::colorWithRed_green_blue_alpha(0.0, 0.5, 1.0, 0.15);
                    fill_color.set();

                    let bezier_class =
                        objc2::runtime::AnyClass::get("NSBezierPath").expect("NSBezierPath exists");
                    let path: *mut AnyObject = msg_send![bezier_class, bezierPathWithRect: view_rect];
                    let _: () = msg_send![path, fill];

                    // Draw selection rectangle border (blue)
                    let stroke_color = NSColor::colorWithRed_green_blue_alpha(0.0, 0.5, 1.0, 1.0);
                    stroke_color.set();

                    let _: () = msg_send![path, setLineWidth: 2.0f64];
                    let _: () = msg_send![path, stroke];
                }
            }
        }

        #[method(acceptsFirstMouse:)]
        fn accepts_first_mouse(&self, _event: *mut AnyObject) -> bool {
            true
        }
    }

    unsafe impl NSObjectProtocol for RegionSelectionView {}
);

impl RegionSelectionView {
    pub(super) fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let alloc = mtm.alloc::<Self>();
        unsafe { msg_send_id![alloc, initWithFrame: frame] }
    }
}
