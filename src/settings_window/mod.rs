//! Settings Window implementation using objc2
//!
//! This module provides a standard macOS window for Vissper settings,
//! including transparency controls for the transcription overlay,
//! transcript storage location configuration, and AI provider credentials.

mod actions;
mod controls;
mod delegate;
mod folder_picker;
mod path_utils;

pub(crate) use delegate::SettingsActionDelegate;

use objc2::msg_send_id;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSScreen, NSSegmentedControl, NSTabView, NSTextField,
    NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use once_cell::sync::OnceCell;
use std::sync::Mutex;
use tracing::info;

use crate::keychain;

/// Named constants for AppKit values and layout dimensions
pub(crate) mod constants {
    use objc2_foundation::CGFloat;

    /// NSBezelStyleRounded constant
    pub const NS_BEZEL_STYLE_ROUNDED: u64 = 1;

    /// NSModalResponseOK constant
    pub const NS_MODAL_RESPONSE_OK: isize = 1;

    /// Window width in points (landscape)
    pub const WINDOW_WIDTH: CGFloat = 550.0;

    /// Window height in points (landscape)
    pub const WINDOW_HEIGHT: CGFloat = 440.0;

    /// Standard padding for UI elements
    pub const PADDING: CGFloat = 20.0;

    /// Tab content height (window height minus title bar and tab bar)
    pub const TAB_CONTENT_HEIGHT: CGFloat = 370.0;
}

use constants::{WINDOW_HEIGHT, WINDOW_WIDTH};

/// Global state for the settings window
static SETTINGS_WINDOW: OnceCell<Mutex<SettingsWindowInner>> = OnceCell::new();

/// Result from creating the settings window with all its controls.
struct WindowCreationResult {
    window: Retained<NSWindow>,
    tab_view: Retained<NSTabView>,
    transparency_value_label: Retained<NSTextField>,
    transcript_path_label: Retained<NSTextField>,
    screenshot_path_label: Retained<NSTextField>,
    provider_selector: Retained<NSSegmentedControl>,
    azure_controls: controls::AzureControls,
    openai_controls: controls::OpenAIControls,
}

/// Inner settings window state holding retained Objective-C references
struct SettingsWindowInner {
    window: Retained<NSWindow>,
    #[allow(dead_code)]
    delegate: Retained<SettingsActionDelegate>,
    #[allow(dead_code)]
    tab_view: Retained<NSTabView>,
    transparency_value_label: Retained<NSTextField>,
    transcript_path_label: Retained<NSTextField>,
    screenshot_path_label: Retained<NSTextField>,
    // Provider selector
    #[allow(dead_code)]
    provider_selector: Retained<NSSegmentedControl>,
    // Azure controls
    azure_endpoint_field: Retained<NSTextField>,
    azure_stt_deployment_field: Retained<NSTextField>,
    azure_polish_deployment_field: Retained<NSTextField>,
    azure_api_key_field: Retained<NSTextField>,
    azure_status_label: Retained<NSTextField>,
    // OpenAI controls
    openai_api_key_field: Retained<NSTextField>,
    openai_status_label: Retained<NSTextField>,
}

// SAFETY: SettingsWindowInner is only accessed from the main thread via
// MainThreadMarker checks. The Retained types are Send when the underlying
// types are MainThreadOnly (which they are for UI objects).
unsafe impl Send for SettingsWindowInner {}

/// Settings window manager.
pub(crate) struct SettingsWindow;

impl SettingsWindow {
    /// Show the settings window, creating it if not already created.
    pub fn show() {
        info!("Opening settings window");

        let mtm = match MainThreadMarker::new() {
            Some(m) => m,
            None => {
                info!("Not on main thread, cannot show settings window");
                return;
            }
        };

        // Activate the application to bring it to front
        let app = NSApplication::sharedApplication(mtm);
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);

        // Check if window already exists
        if let Some(inner) = SETTINGS_WINDOW.get() {
            if let Ok(inner) = inner.lock() {
                inner.window.makeKeyAndOrderFront(None);
                return;
            }
        }

        // Create delegate for control actions
        let delegate = SettingsActionDelegate::new(mtm);

        // Create new window with UI
        let result = Self::create_window(mtm, &delegate);

        // Store in global state
        let inner = SettingsWindowInner {
            window: result.window,
            delegate,
            tab_view: result.tab_view,
            transparency_value_label: result.transparency_value_label,
            transcript_path_label: result.transcript_path_label,
            screenshot_path_label: result.screenshot_path_label,
            provider_selector: result.provider_selector,
            azure_endpoint_field: result.azure_controls.endpoint_field,
            azure_stt_deployment_field: result.azure_controls.stt_deployment_field,
            azure_polish_deployment_field: result.azure_controls.polish_deployment_field,
            azure_api_key_field: result.azure_controls.api_key_field,
            azure_status_label: result.azure_controls.status_label,
            openai_api_key_field: result.openai_controls.api_key_field,
            openai_status_label: result.openai_controls.status_label,
        };
        if SETTINGS_WINDOW.set(Mutex::new(inner)).is_err() {
            // Window was created by another thread, show that one instead
            if let Some(inner) = SETTINGS_WINDOW.get() {
                if let Ok(inner) = inner.lock() {
                    inner.window.makeKeyAndOrderFront(None);
                }
            }
        }
    }

    /// Create the settings window with all UI sections organized in tabs.
    fn create_window(
        mtm: MainThreadMarker,
        delegate: &SettingsActionDelegate,
    ) -> WindowCreationResult {
        // Get main screen dimensions for centering
        let main_screen = NSScreen::mainScreen(mtm);
        let screen_frame = match main_screen {
            Some(screen) => screen.frame(),
            None => NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1920.0, 1080.0)),
        };

        // Center the window on screen
        let origin_x = (screen_frame.size.width - WINDOW_WIDTH) / 2.0;
        let origin_y = (screen_frame.size.height - WINDOW_HEIGHT) / 2.0;

        let frame = NSRect::new(
            NSPoint::new(origin_x, origin_y),
            NSSize::new(WINDOW_WIDTH, WINDOW_HEIGHT),
        );

        // Create standard macOS window with title bar
        let style_mask = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Miniaturizable;

        // SAFETY: NSWindow initialization with valid parameters on main thread
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                frame,
                style_mask,
                NSBackingStoreType::NSBackingStoreBuffered,
                false,
            )
        };

        window.setTitle(&NSString::from_str("Vissper Settings"));
        unsafe { window.setReleasedWhenClosed(false) };

        // Create content view
        let content_frame = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(WINDOW_WIDTH, WINDOW_HEIGHT),
        );

        // SAFETY: NSView initialization with valid frame on main thread
        let content_view: Retained<NSView> =
            unsafe { msg_send_id![mtm.alloc::<NSView>(), initWithFrame: content_frame] };

        // Create tab view that fills the content area with padding
        let tab_frame = NSRect::new(
            NSPoint::new(10.0, 10.0),
            NSSize::new(WINDOW_WIDTH - 20.0, WINDOW_HEIGHT - 20.0),
        );
        let tab_view = controls::create_tab_view(mtm, tab_frame);

        // Create "General" tab
        let general_tab = controls::create_tab_item(mtm, "General");

        // Create content view for General tab
        let general_content: Retained<NSView> = unsafe {
            msg_send_id![mtm.alloc::<NSView>(), initWithFrame: NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(WINDOW_WIDTH - 40.0, constants::TAB_CONTENT_HEIGHT)
            )]
        };

        // Add General tab controls
        let (_slider, transparency_value_label) =
            controls::add_transparency_controls(mtm, &general_content, delegate);

        let sep1 = controls::create_separator(mtm, 265.0, WINDOW_WIDTH - 40.0);
        unsafe { general_content.addSubview(&sep1) };

        let _segmented_control = controls::add_background_controls(mtm, &general_content, delegate);

        let sep2 = controls::create_separator(mtm, 195.0, WINDOW_WIDTH - 40.0);
        unsafe { general_content.addSubview(&sep2) };

        let transcript_path = path_utils::get_transcript_display_path();
        let transcript_path_label =
            controls::add_location_controls(mtm, &general_content, delegate, &transcript_path);

        let sep3 = controls::create_separator(mtm, 125.0, WINDOW_WIDTH - 40.0);
        unsafe { general_content.addSubview(&sep3) };

        let screenshot_path = path_utils::get_screenshot_display_path();
        let screenshot_path_label = controls::add_screenshot_location_controls(
            mtm,
            &general_content,
            delegate,
            &screenshot_path,
        );

        // Add provider selector at the bottom of General tab (below Screenshot Location which ends at y=75)
        let sep4 = controls::create_separator(mtm, 55.0, WINDOW_WIDTH - 40.0);
        unsafe { general_content.addSubview(&sep4) };

        let provider_selector =
            actions::create_provider_selector(mtm, &general_content, delegate);

        unsafe { general_tab.setView(Some(&general_content)) };

        // Create "Azure" tab
        let azure_tab = controls::create_tab_item(mtm, "Azure OpenAI");

        // Create content view for Azure tab
        let azure_content: Retained<NSView> = unsafe {
            msg_send_id![mtm.alloc::<NSView>(), initWithFrame: NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(WINDOW_WIDTH - 40.0, constants::TAB_CONTENT_HEIGHT)
            )]
        };

        // Add Azure tab controls
        let azure_credentials = keychain::get_azure_credentials().ok();
        let azure_controls =
            controls::add_azure_controls(mtm, &azure_content, delegate, azure_credentials.as_ref());

        unsafe { azure_tab.setView(Some(&azure_content)) };

        // Create "OpenAI" tab
        let openai_tab = controls::create_tab_item(mtm, "OpenAI");

        // Create content view for OpenAI tab
        let openai_content: Retained<NSView> = unsafe {
            msg_send_id![mtm.alloc::<NSView>(), initWithFrame: NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(WINDOW_WIDTH - 40.0, constants::TAB_CONTENT_HEIGHT)
            )]
        };

        // Add OpenAI tab controls
        let openai_credentials = keychain::get_openai_credentials().ok();
        let openai_controls = controls::add_openai_controls(
            mtm,
            &openai_content,
            delegate,
            openai_credentials.as_ref(),
        );

        unsafe { openai_tab.setView(Some(&openai_content)) };

        // Add tabs to tab view
        unsafe {
            tab_view.addTabViewItem(&general_tab);
            tab_view.addTabViewItem(&azure_tab);
            tab_view.addTabViewItem(&openai_tab);
        }

        // Add tab view to content view
        unsafe { content_view.addSubview(&tab_view) };

        window.setContentView(Some(&content_view));
        window.makeKeyAndOrderFront(None);

        info!("Settings window created and shown");

        WindowCreationResult {
            window,
            tab_view,
            transparency_value_label,
            transcript_path_label,
            screenshot_path_label,
            provider_selector,
            azure_controls,
            openai_controls,
        }
    }

    /// Update the transparency value label.
    pub(super) fn update_transparency_label(value: f64) {
        if let Some(inner) = SETTINGS_WINDOW.get() {
            if let Ok(inner) = inner.lock() {
                let percentage = (value * 100.0).round() as i32;
                unsafe {
                    inner
                        .transparency_value_label
                        .setStringValue(&NSString::from_str(&format!("{}%", percentage)));
                }
            }
        }
    }

    /// Show the folder picker dialog for selecting transcript location.
    pub(super) fn show_folder_picker() {
        actions::show_folder_picker();
    }

    /// Reset transcript location to default.
    pub(super) fn reset_transcript_location() {
        actions::reset_transcript_location();
    }

    /// Show the folder picker dialog for selecting screenshot location.
    pub(super) fn show_screenshot_folder_picker() {
        actions::show_screenshot_folder_picker();
    }

    /// Reset screenshot location to default.
    pub(super) fn reset_screenshot_location() {
        actions::reset_screenshot_location();
    }

    /// Save Azure credentials from the UI fields to keychain.
    pub(super) fn save_azure_credentials() {
        actions::save_azure_credentials();
    }

    /// Clear Azure credentials from keychain.
    pub(super) fn clear_azure_credentials() {
        actions::clear_azure_credentials();
    }

    /// Save OpenAI credentials from the UI fields to keychain.
    pub(super) fn save_openai_credentials() {
        actions::save_openai_credentials();
    }

    /// Clear OpenAI credentials from keychain.
    pub(super) fn clear_openai_credentials() {
        actions::clear_openai_credentials();
    }

    /// Handle AI provider selection change.
    pub(super) fn handle_provider_selection(selected_segment: isize) {
        actions::handle_provider_selection(selected_segment);
    }

    /// Hide the settings window.
    #[allow(dead_code)]
    pub fn hide() {
        if let Some(inner) = SETTINGS_WINDOW.get() {
            if let Ok(inner) = inner.lock() {
                inner.window.orderOut(None);
                info!("Settings window hidden");
            }
        }
    }

    /// Close the settings window.
    #[allow(dead_code)]
    pub fn close() {
        if let Some(inner) = SETTINGS_WINDOW.get() {
            if let Ok(inner) = inner.lock() {
                inner.window.close();
                info!("Settings window closed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::constants;

    #[test]
    fn test_constants() {
        assert_eq!(constants::NS_BEZEL_STYLE_ROUNDED, 1);
        assert_eq!(constants::NS_MODAL_RESPONSE_OK, 1);
        assert_eq!(constants::WINDOW_WIDTH, 550.0);
        assert_eq!(constants::WINDOW_HEIGHT, 440.0);
        assert_eq!(constants::PADDING, 20.0);
        assert_eq!(constants::TAB_CONTENT_HEIGHT, 370.0);
    }
}
