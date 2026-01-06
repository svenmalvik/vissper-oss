//! Region selection screenshot module
//!
//! Provides an interactive overlay for selecting a screen region
//! to capture via screencapture -R x,y,w,h command.

mod state;
mod view;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, msg_send_id};
use objc2_app_kit::{NSBackingStoreType, NSColor, NSWindow, NSWindowStyleMask};
use objc2_foundation::{MainThreadMarker, NSOperationQueue};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};

use crate::recording::RecordingSession;
use crate::screenshot;

use state::{RegionSelectionState, MIN_SELECTION_SIZE, OVERLAY_WINDOW_LEVEL, REGION_STATE};
use view::RegionSelectionView;

/// Region selection manager
pub(crate) struct RegionSelection;

impl RegionSelection {
    /// Start region selection mode
    ///
    /// Shows a transparent overlay on all screens. User can click and drag
    /// to select a region. Press ESC to cancel.
    ///
    /// Safe to call from any thread - dispatches to main thread if needed.
    pub(crate) fn start(recording_state: Arc<Mutex<Option<RecordingSession>>>) {
        debug!("Starting region selection");

        // Check if already active
        if let Ok(guard) = REGION_STATE.lock() {
            if guard.is_some() {
                debug!("Region selection already active, ignoring");
                return;
            }
        }

        // If already on main thread, start directly
        if let Some(mtm) = MainThreadMarker::new() {
            Self::start_on_main_thread(mtm, recording_state);
            return;
        }

        // Not on main thread - dispatch to main queue
        debug!("Dispatching region selection to main thread");
        let block = RcBlock::new(move || {
            if let Some(mtm) = MainThreadMarker::new() {
                Self::start_on_main_thread(mtm, recording_state.clone());
            }
        });

        unsafe {
            let queue = NSOperationQueue::mainQueue();
            let _: () = msg_send![&queue, addOperationWithBlock: &*block];
        }
    }

    /// Internal: start region selection on main thread
    fn start_on_main_thread(
        mtm: MainThreadMarker,
        recording_state: Arc<Mutex<Option<RecordingSession>>>,
    ) {
        info!("Creating region selection overlay");

        // Get main screen (simplified approach - works on primary monitor)
        let ns_screen_class =
            objc2::runtime::AnyClass::get("NSScreen").expect("NSScreen class must exist");
        let main_screen: *mut AnyObject = unsafe { msg_send![ns_screen_class, mainScreen] };
        if main_screen.is_null() {
            error!("No main screen found");
            return;
        }

        let screen_frame: NSRect = unsafe { msg_send![main_screen, frame] };
        let max_y = screen_frame.origin.y + screen_frame.size.height;

        // Create overlay window for main screen
        let mut windows = Vec::new();
        let window = Self::create_overlay_window(mtm, screen_frame);
        windows.push(window);

        // Setup ESC key monitoring
        let event_monitor = Self::setup_keyboard_monitor();

        // Store state
        let state = RegionSelectionState {
            windows,
            selection_origin: None,
            current_rect: None,
            event_monitor,
            total_screen_height: max_y,
            recording_state,
        };

        if let Ok(mut guard) = REGION_STATE.lock() {
            *guard = Some(state);
        }

        info!("Region selection overlay active - drag to select, ESC to cancel");
    }

    /// Create a single overlay window for a screen
    fn create_overlay_window(mtm: MainThreadMarker, frame: NSRect) -> Retained<NSWindow> {
        // Create borderless fullscreen window
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                frame,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::NSBackingStoreBuffered,
                false,
            )
        };

        // Configure window
        unsafe { window.setReleasedWhenClosed(false) };
        window.setOpaque(false);
        window.setLevel(OVERLAY_WINDOW_LEVEL);

        // Semi-transparent dark overlay
        let bg_color = unsafe { NSColor::colorWithRed_green_blue_alpha(0.0, 0.0, 0.0, 0.3) };
        window.setBackgroundColor(Some(&bg_color));

        // Accept mouse events
        unsafe {
            let _: () = msg_send![&window, setIgnoresMouseEvents: false];
            let _: () = msg_send![&window, setAcceptsMouseMovedEvents: true];
        }

        // Create custom view for mouse handling
        let view_frame = NSRect::new(NSPoint::new(0.0, 0.0), frame.size);
        let view = RegionSelectionView::new(mtm, view_frame);
        window.setContentView(Some(&view));

        // Make window key and bring to front
        window.makeKeyAndOrderFront(None);

        window
    }

    /// Setup keyboard monitor for ESC key
    fn setup_keyboard_monitor() -> Option<Retained<AnyObject>> {
        // NSEventMaskKeyDown = 1 << 10 = 1024
        let mask: u64 = 1024;

        let block = RcBlock::new(|event: *mut AnyObject| -> *mut AnyObject {
            if event.is_null() {
                return event;
            }

            // Get keyCode from event
            let keycode: u16 = unsafe { msg_send![event, keyCode] };

            // ESC key = keycode 53
            if keycode == 53 {
                debug!("ESC pressed - canceling region selection");
                RegionSelection::cleanup();
                return std::ptr::null_mut(); // Consume event
            }

            event // Pass through other events
        });

        let monitor: Option<Retained<AnyObject>> = unsafe {
            let ns_event_class = objc2::runtime::AnyClass::get("NSEvent")?;
            msg_send_id![
                ns_event_class,
                addLocalMonitorForEventsMatchingMask: mask
                handler: &*block
            ]
        };

        monitor
    }

    /// Remove keyboard monitor
    fn remove_keyboard_monitor() {
        if let Ok(mut guard) = REGION_STATE.lock() {
            if let Some(ref mut state) = *guard {
                if let Some(monitor) = state.event_monitor.take() {
                    unsafe {
                        if let Some(ns_event_class) = objc2::runtime::AnyClass::get("NSEvent") {
                            let _: () = msg_send![ns_event_class, removeMonitor: &*monitor];
                        }
                    }
                    debug!("Keyboard monitor removed");
                }
            }
        }
    }

    /// Update selection origin when mouse down
    pub(super) fn set_selection_origin(point: NSPoint) {
        if let Ok(mut guard) = REGION_STATE.lock() {
            if let Some(ref mut state) = *guard {
                state.selection_origin = Some(point);
                state.current_rect = None;
            }
        }
    }

    /// Update current selection rectangle during drag
    pub(super) fn update_selection(current_point: NSPoint) {
        if let Ok(mut guard) = REGION_STATE.lock() {
            if let Some(ref mut state) = *guard {
                if let Some(origin) = state.selection_origin {
                    // Calculate rectangle (handle negative width/height)
                    let x = origin.x.min(current_point.x);
                    let y = origin.y.min(current_point.y);
                    let w = (current_point.x - origin.x).abs();
                    let h = (current_point.y - origin.y).abs();

                    state.current_rect = Some(NSRect::new(NSPoint::new(x, y), NSSize::new(w, h)));
                }
            }
        }
    }

    /// Get current selection rectangle (if any)
    pub(super) fn get_current_rect() -> Option<NSRect> {
        if let Ok(guard) = REGION_STATE.lock() {
            if let Some(ref state) = *guard {
                return state.current_rect;
            }
        }
        None
    }

    /// Get total screen height for coordinate conversion
    fn get_total_screen_height() -> f64 {
        if let Ok(guard) = REGION_STATE.lock() {
            if let Some(ref state) = *guard {
                return state.total_screen_height;
            }
        }
        0.0
    }

    /// Get recording state for inserting screenshot reference
    fn get_recording_state() -> Option<Arc<Mutex<Option<RecordingSession>>>> {
        if let Ok(guard) = REGION_STATE.lock() {
            if let Some(ref state) = *guard {
                return Some(state.recording_state.clone());
            }
        }
        None
    }

    /// Complete selection and capture region
    pub(super) fn complete_selection() {
        let rect = Self::get_current_rect();
        let screen_height = Self::get_total_screen_height();
        let recording_state = Self::get_recording_state();

        // Cleanup first (closes overlay)
        Self::cleanup();

        // Validate selection before spawning capture thread
        let rect = match rect {
            Some(r) => r,
            None => return,
        };

        // Check minimum size
        if rect.size.width < MIN_SELECTION_SIZE || rect.size.height < MIN_SELECTION_SIZE {
            debug!(
                "Selection too small ({:.0}x{:.0}), ignoring",
                rect.size.width, rect.size.height
            );
            return;
        }

        // Convert macOS coordinates (bottom-left origin) to screencapture (top-left origin)
        let x = rect.origin.x;
        let y = screen_height - rect.origin.y - rect.size.height;
        let width = rect.size.width;
        let height = rect.size.height;

        // Spawn background thread to capture after overlay is gone
        // This allows the main thread to process window close events
        std::thread::spawn(move || {
            // Wait for main thread to process window close
            std::thread::sleep(std::time::Duration::from_millis(150));

            info!(
                "Capturing region: x={:.0}, y={:.0}, w={:.0}, h={:.0}",
                x, y, width, height
            );

            // Capture the region first, then show flash
            match screenshot::capture_region_screenshot(x, y, width, height) {
                Ok(filename) => {
                    info!("Region screenshot captured: {}", filename);

                    // Show flash effect after capture
                    crate::screenshot_flash::ScreenshotFlash::show();

                    // Insert screenshot reference into transcript if recording
                    if let Some(recording_state) = recording_state {
                        if let Ok(state) = recording_state.lock() {
                            if let Some(ref session) = *state {
                                if let Ok(mut session_data) = session.session_data.lock() {
                                    let relative_path = format!("screenshots/{}", filename);
                                    session_data.insert_screenshot(&relative_path);
                                    info!("Screenshot reference inserted into transcript");
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to capture region screenshot: {}", e);
                }
            }
        });
    }

    /// Cleanup: close windows and remove monitor
    fn cleanup() {
        debug!("Cleaning up region selection");

        // Remove keyboard monitor first
        Self::remove_keyboard_monitor();

        // Close all windows - use orderOut first to immediately hide
        if let Ok(mut guard) = REGION_STATE.lock() {
            if let Some(mut state) = guard.take() {
                for window in state.windows.drain(..) {
                    // orderOut immediately removes window from screen (no animation)
                    window.orderOut(None);
                    window.close();
                }
                debug!("Region selection overlay closed");
            }
        }
    }

    /// Request redraw of all overlay windows
    pub(super) fn request_redraw() {
        if let Ok(guard) = REGION_STATE.lock() {
            if let Some(ref state) = *guard {
                for window in &state.windows {
                    if let Some(content_view) = window.contentView() {
                        unsafe {
                            let _: () = msg_send![&content_view, setNeedsDisplay: true];
                        }
                    }
                }
            }
        }
    }
}
