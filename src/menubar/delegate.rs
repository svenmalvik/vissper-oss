//! Menu delegate for handling Objective-C callbacks
//!
//! Defines the VissperMenuDelegate class that handles menu item actions.

use objc2::rc::Retained;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{NSAlert, NSAlertStyle};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSString};
use tracing::info;

use super::{MenuBar, CALLBACKS};

/// Version from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");

// Define the delegate class for handling menu actions
declare_class!(
    pub(super) struct VissperMenuDelegate;

    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - Main thread only mutability is correct for menu delegates.
    // - `VissperMenuDelegate` does not implement `Drop`.
    unsafe impl ClassType for VissperMenuDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "VissperMenuDelegate";
    }

    impl DeclaredClass for VissperMenuDelegate {}

    unsafe impl VissperMenuDelegate {
        #[method(handleStartRecording:)]
        fn handle_start_recording(&self, _sender: *mut NSObject) {
            info!("Start Recording menu item clicked");
            if let Some(callbacks) = CALLBACKS.get() {
                (callbacks.on_start_recording)();
            }
        }

        #[method(handleStopNoPolish:)]
        fn handle_stop_no_polish(&self, _sender: *mut NSObject) {
            info!("Stop Recording (No polishing) clicked");
            if let Some(callbacks) = CALLBACKS.get() {
                (callbacks.on_stop_no_polish)();
            }
        }

        #[method(handleStopBasicPolish:)]
        fn handle_stop_basic_polish(&self, _sender: *mut NSObject) {
            info!("Stop Recording (Basic polishing) clicked");
            if let Some(callbacks) = CALLBACKS.get() {
                (callbacks.on_stop_basic_polish)();
            }
        }

        #[method(handleStopMeetingNotes:)]
        fn handle_stop_meeting_notes(&self, _sender: *mut NSObject) {
            info!("Stop Recording (Meeting notes) clicked");
            if let Some(callbacks) = CALLBACKS.get() {
                (callbacks.on_stop_meeting_notes)();
            }
        }

        #[method(handleShowWindow:)]
        fn handle_show_window(&self, _sender: *mut NSObject) {
            info!("Show window menu item clicked");
            if let Some(callbacks) = CALLBACKS.get() {
                (callbacks.on_show_window)();
            }
        }

        #[method(handleScreenshot:)]
        fn handle_screenshot(&self, _sender: *mut NSObject) {
            info!("Capture Entire Screen clicked");
            if let Some(callbacks) = CALLBACKS.get() {
                (callbacks.on_screenshot)();
            }
        }

        #[method(handleRegionScreenshot:)]
        fn handle_region_screenshot(&self, _sender: *mut NSObject) {
            info!("Capture Selected Area clicked");
            if let Some(callbacks) = CALLBACKS.get() {
                (callbacks.on_region_screenshot)();
            }
        }

        #[method(handleSettings:)]
        fn handle_settings(&self, _sender: *mut NSObject) {
            info!("Settings menu item clicked");
            if let Some(callbacks) = CALLBACKS.get() {
                (callbacks.on_settings)();
            }
        }

        #[method(handleQuit:)]
        fn handle_quit(&self, _sender: *mut NSObject) {
            info!("Quit menu item clicked");
            if let Some(callbacks) = CALLBACKS.get() {
                (callbacks.on_quit)();
            }
            MenuBar::stop();
        }

        #[method(handleLanguageEnglish:)]
        fn handle_language_english(&self, _sender: *mut NSObject) {
            info!("Language English selected");
            MenuBar::set_language("en");
        }

        #[method(handleLanguageNorwegian:)]
        fn handle_language_norwegian(&self, _sender: *mut NSObject) {
            info!("Language Norwegian selected");
            MenuBar::set_language("no");
        }

        #[method(handleLanguageDanish:)]
        fn handle_language_danish(&self, _sender: *mut NSObject) {
            info!("Language Danish selected");
            MenuBar::set_language("da");
        }

        #[method(handleLanguageFinnish:)]
        fn handle_language_finnish(&self, _sender: *mut NSObject) {
            info!("Language Finnish selected");
            MenuBar::set_language("fi");
        }

        #[method(handleLanguageGerman:)]
        fn handle_language_german(&self, _sender: *mut NSObject) {
            info!("Language German selected");
            MenuBar::set_language("de");
        }

        #[method(handleAbout:)]
        fn handle_about(&self, _sender: *mut NSObject) {
            info!("About menu item clicked");
            if let Some(mtm) = MainThreadMarker::new() {
                unsafe {
                    let alert = NSAlert::new(mtm);
                    alert.setAlertStyle(NSAlertStyle::Informational);
                    let title = NSString::from_str("Vissper");
                    alert.setMessageText(&title);
                    let message = NSString::from_str(&format!(
                        "Version {}\n\n© 2025 Vissper. All rights reserved.\n\nReal-time transcription and AI summaries for your meetings.",
                        VERSION
                    ));
                    alert.setInformativeText(&message);
                    alert.runModal();
                }
            }
        }

        #[method(handleUpdateAvailable:)]
        fn handle_update_available(&self, _sender: *mut NSObject) {
            info!("Update available menu item clicked");
            if let Some(callbacks) = CALLBACKS.get() {
                (callbacks.on_update_available)();
            }
        }
    }

    unsafe impl NSObjectProtocol for VissperMenuDelegate {}
);

impl VissperMenuDelegate {
    pub(super) fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let alloc = mtm.alloc::<Self>();
        unsafe { msg_send_id![alloc, init] }
    }
}
