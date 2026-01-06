//! Screenshot flash effect module
//!
//! Provides a brief white screen flash when taking screenshots,
//! mimicking the native macOS screenshot feedback.

use block2::RcBlock;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2_app_kit::{NSBackingStoreType, NSColor, NSScreen, NSWindow, NSWindowStyleMask};
use objc2_foundation::{MainThreadMarker, NSOperationQueue, NSPoint, NSRect, NSSize};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tracing::debug;

/// Window level high enough to appear above all content (screen saver level).
const FLASH_WINDOW_LEVEL: isize = 1000;

/// Global state for the flash window.
/// Only accessed from main thread after initial setup.
static FLASH_STATE: Lazy<Mutex<Option<FlashState>>> = Lazy::new(|| Mutex::new(None));

/// Inner state holding the flash window reference.
struct FlashState {
    window: Retained<NSWindow>,
}

// SAFETY: FlashState is only accessed from the main thread via MainThreadMarker checks.
// The Retained types are Send when the underlying types are MainThreadOnly.
unsafe impl Send for FlashState {}

/// Screenshot flash manager.
///
/// Provides a brief white overlay flash effect when screenshots are taken,
/// similar to the native macOS screenshot feedback.
pub(crate) struct ScreenshotFlash;

impl ScreenshotFlash {
    /// Show the screenshot flash effect.
    ///
    /// Creates a fullscreen white overlay that fades in and out quickly.
    /// Safe to call from any thread - dispatches to main thread if needed.
    pub(crate) fn show() {
        debug!("Triggering screenshot flash");

        // If already on main thread, show directly
        if let Some(mtm) = MainThreadMarker::new() {
            Self::show_on_main_thread(mtm);
            return;
        }

        // Not on main thread - dispatch to main queue
        debug!("Dispatching flash to main thread");
        let block = RcBlock::new(|| {
            if let Some(mtm) = MainThreadMarker::new() {
                Self::show_on_main_thread(mtm);
            }
        });

        // SAFETY: NSOperationQueue::mainQueue() is safe to call from any thread.
        // addOperationWithBlock: is a standard Objective-C method that schedules
        // the block to run on the main thread.
        unsafe {
            let queue = NSOperationQueue::mainQueue();
            let _: () = msg_send![&queue, addOperationWithBlock: &*block];
        }
    }

    /// Internal: show flash on main thread (requires MainThreadMarker).
    fn show_on_main_thread(mtm: MainThreadMarker) {
        // Get main screen dimensions
        let main_screen = NSScreen::mainScreen(mtm);
        let screen_frame = match main_screen {
            Some(screen) => screen.frame(),
            None => {
                debug!("No main screen found, skipping flash");
                return;
            }
        };

        // Create fullscreen frame
        let frame = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(screen_frame.size.width, screen_frame.size.height),
        );

        // SAFETY: NSWindow initialization with valid frame, style mask, and backing store type.
        // MainThreadMarker guarantees we're on the main thread as required by AppKit.
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                frame,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::NSBackingStoreBuffered,
                false,
            )
        };

        // Configure window for flash effect
        // SAFETY: setReleasedWhenClosed is a standard NSWindow configuration call.
        unsafe { window.setReleasedWhenClosed(false) };
        window.setOpaque(false);

        // Start with peak brightness immediately (0.6 alpha white)
        // SAFETY: NSColor class method for creating RGBA color.
        let bg_color = unsafe { NSColor::colorWithRed_green_blue_alpha(1.0, 1.0, 1.0, 0.6) };
        window.setBackgroundColor(Some(&bg_color));

        // Set high window level to appear above everything
        window.setLevel(FLASH_WINDOW_LEVEL);

        // Ignore mouse events (click-through)
        // SAFETY: setIgnoresMouseEvents is a standard NSWindow method.
        unsafe {
            let _: () = msg_send![&window, setIgnoresMouseEvents: true];
        }

        // Store in global state and show window
        let state = FlashState { window };
        if let Ok(mut guard) = FLASH_STATE.lock() {
            // Close any existing flash window first
            if let Some(old_state) = guard.take() {
                old_state.window.close();
            }
            // Show the new window
            state.window.orderFront(None);
            *guard = Some(state);
        }

        // Start animation sequence
        Self::start_animation();
    }

    /// Start the fade-out animation using a single background thread.
    fn start_animation() {
        std::thread::spawn(|| {
            // Phase 1: Wait 100ms at peak brightness
            std::thread::sleep(std::time::Duration::from_millis(100));
            Self::dispatch_alpha_update(0.4);

            // Phase 2: Fade to 0.2 alpha after 50ms
            std::thread::sleep(std::time::Duration::from_millis(50));
            Self::dispatch_alpha_update(0.2);

            // Phase 3: Fade to 0.05 alpha after 50ms
            std::thread::sleep(std::time::Duration::from_millis(50));
            Self::dispatch_alpha_update(0.05);

            // Phase 4: Close window after 50ms
            std::thread::sleep(std::time::Duration::from_millis(50));
            Self::dispatch_close();
        });
    }

    /// Dispatch an alpha update to the main thread.
    fn dispatch_alpha_update(alpha: f64) {
        let block = RcBlock::new(move || {
            if let Ok(guard) = FLASH_STATE.lock() {
                if let Some(ref state) = *guard {
                    // SAFETY: NSColor class method and setBackgroundColor are safe on main thread.
                    unsafe {
                        let color = NSColor::colorWithRed_green_blue_alpha(1.0, 1.0, 1.0, alpha);
                        state.window.setBackgroundColor(Some(&color));
                    }
                }
            }
        });

        // SAFETY: NSOperationQueue::mainQueue() is safe to call from any thread.
        unsafe {
            let queue = NSOperationQueue::mainQueue();
            let _: () = msg_send![&queue, addOperationWithBlock: &*block];
        }
    }

    /// Dispatch window close to the main thread.
    fn dispatch_close() {
        let block = RcBlock::new(|| {
            if let Ok(mut guard) = FLASH_STATE.lock() {
                if let Some(state) = guard.take() {
                    state.window.close();
                    debug!("Screenshot flash completed");
                }
            }
        });

        // SAFETY: NSOperationQueue::mainQueue() is safe to call from any thread.
        unsafe {
            let queue = NSOperationQueue::mainQueue();
            let _: () = msg_send![&queue, addOperationWithBlock: &*block];
        }
    }
}
