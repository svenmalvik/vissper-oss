//! Global hotkey management
//!
//! Provides global keyboard shortcuts for quick actions.
//! Hotkeys work even when the app is in the background.

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

/// Initialize global hotkeys for the application
///
/// Currently registered hotkeys:
/// - Control + Space: Start recording / Stop with no polishing
/// - Control + Shift + 1: Start recording / Stop with basic polishing
/// - Control + Shift + 2: Stop with meeting notes
/// - Control + Shift + 0: Take screenshot (only during recording)
/// - Control + Shift + 9: Region screenshot (select area with mouse)
pub(crate) fn init_hotkeys() -> Result<GlobalHotKeyManager, String> {
    let manager = GlobalHotKeyManager::new()
        .map_err(|e| format!("Failed to create hotkey manager: {}", e))?;

    // Control + Space: Start recording / Stop with no polishing
    let no_polish_hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::Space);

    manager
        .register(no_polish_hotkey)
        .map_err(|e| format!("Failed to register no polish hotkey: {}", e))?;

    info!("Registered global hotkey: Control + Space (no polishing)");

    // Control + Shift + 1: Start recording / Stop with basic polishing
    let basic_polish_hotkey =
        HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit1);

    manager
        .register(basic_polish_hotkey)
        .map_err(|e| format!("Failed to register basic polish hotkey: {}", e))?;

    info!("Registered global hotkey: Control + Shift + 1 (basic polishing)");

    // Control + Shift + 2: Stop with meeting notes
    let meeting_notes_hotkey =
        HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit2);

    manager
        .register(meeting_notes_hotkey)
        .map_err(|e| format!("Failed to register meeting notes hotkey: {}", e))?;

    info!("Registered global hotkey: Control + Shift + 2 (meeting notes)");

    // Control + Shift + 0: Take screenshot
    let screenshot_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit0);

    manager
        .register(screenshot_hotkey)
        .map_err(|e| format!("Failed to register screenshot hotkey: {}", e))?;

    info!("Registered global hotkey: Control + Shift + 0 (screenshot)");

    // Control + Shift + 9: Region screenshot (select area with mouse)
    let region_screenshot_hotkey =
        HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit9);

    manager
        .register(region_screenshot_hotkey)
        .map_err(|e| format!("Failed to register region screenshot hotkey: {}", e))?;

    info!("Registered global hotkey: Control + Shift + 9 (region screenshot)");

    Ok(manager)
}

/// Get the hotkey ID for no polishing (Control + Space)
fn no_polish_hotkey_id() -> u32 {
    let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::Space);
    hotkey.id()
}

/// Get the hotkey ID for basic polishing (Control + Shift + 1)
fn basic_polish_hotkey_id() -> u32 {
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit1);
    hotkey.id()
}

/// Get the hotkey ID for meeting notes (Control + Shift + 2)
fn meeting_notes_hotkey_id() -> u32 {
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit2);
    hotkey.id()
}

/// Get the hotkey ID for screenshot (Control + Shift + 0)
fn screenshot_hotkey_id() -> u32 {
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit0);
    hotkey.id()
}

/// Get the hotkey ID for region screenshot (Control + Shift + 9)
fn region_screenshot_hotkey_id() -> u32 {
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit9);
    hotkey.id()
}

/// Start listening for hotkey events
///
/// This spawns a background thread (not tokio task) that polls for hotkey events
/// and dispatches callbacks to the main thread via dispatch queue.
///
/// # Arguments
/// * `on_no_polish` - Callback for Control + Space (no polishing)
/// * `on_basic_polish` - Callback for Control + Shift + 1 (basic polishing)
/// * `on_meeting_notes` - Callback for Control + Shift + 2 (meeting notes)
/// * `on_screenshot` - Callback for Control + Shift + 0 (screenshot during recording)
/// * `on_region_screenshot` - Callback for Control + Shift + 9 (region screenshot)
pub(crate) fn start_hotkey_listener(
    on_no_polish: Arc<dyn Fn() + Send + Sync>,
    on_basic_polish: Arc<dyn Fn() + Send + Sync>,
    on_meeting_notes: Arc<dyn Fn() + Send + Sync>,
    on_screenshot: Arc<dyn Fn() + Send + Sync>,
    on_region_screenshot: Arc<dyn Fn() + Send + Sync>,
) {
    let no_polish_id = no_polish_hotkey_id();
    let basic_polish_id = basic_polish_hotkey_id();
    let meeting_notes_id = meeting_notes_hotkey_id();
    let screenshot_id = screenshot_hotkey_id();
    let region_screenshot_id = region_screenshot_hotkey_id();

    std::thread::spawn(move || {
        let receiver = GlobalHotKeyEvent::receiver();

        info!("Hotkey listener started on dedicated thread");

        loop {
            // Use try_recv with sleep to avoid blocking issues
            match receiver.try_recv() {
                Ok(event) => {
                    info!("Hotkey event received: {:?}", event);

                    // Only handle key press, ignore key release
                    if event.state != HotKeyState::Pressed {
                        continue;
                    }

                    // Determine which hotkey was pressed and dispatch appropriate callback
                    if event.id == no_polish_id {
                        let callback = on_no_polish.clone();
                        dispatch::Queue::main().exec_async(move || {
                            (callback)();
                        });
                    } else if event.id == basic_polish_id {
                        let callback = on_basic_polish.clone();
                        dispatch::Queue::main().exec_async(move || {
                            (callback)();
                        });
                    } else if event.id == meeting_notes_id {
                        let callback = on_meeting_notes.clone();
                        dispatch::Queue::main().exec_async(move || {
                            (callback)();
                        });
                    } else if event.id == screenshot_id {
                        let callback = on_screenshot.clone();
                        dispatch::Queue::main().exec_async(move || {
                            (callback)();
                        });
                    } else if event.id == region_screenshot_id {
                        let callback = on_region_screenshot.clone();
                        dispatch::Queue::main().exec_async(move || {
                            (callback)();
                        });
                    }
                }
                Err(_) => {
                    // No event, sleep briefly to avoid busy-waiting
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    });
}
