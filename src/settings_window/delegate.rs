//! Settings window delegate for handling control actions
//!
//! Contains the Objective-C delegate class that receives and handles
//! user interactions with settings window controls.

use objc2::rc::Retained;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{NSSegmentedControl, NSSlider};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol};
use tracing::error;

use super::SettingsWindow;
use crate::preferences;
use crate::transcription_window::TranscriptionWindow;

// Delegate class for handling settings control actions
declare_class!(
    /// Objective-C delegate class for settings window control actions.
    ///
    /// This class is registered with the Objective-C runtime and receives
    /// action messages from UI controls in the settings window.
    pub(crate) struct SettingsActionDelegate;

    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - Main thread only mutability is correct for UI delegates.
    // - `SettingsActionDelegate` does not implement `Drop`.
    unsafe impl ClassType for SettingsActionDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "VissperSettingsActionDelegate";
    }

    impl DeclaredClass for SettingsActionDelegate {}

    // SAFETY: All methods are called by AppKit on the main thread,
    // which is guaranteed by MainThreadOnly mutability.
    unsafe impl SettingsActionDelegate {
        /// Handle transparency slider value changes
        #[method(handleTransparencySlider:)]
        fn handle_transparency_slider(&self, sender: *mut NSSlider) {
            // SAFETY: sender is a valid NSSlider passed by AppKit, doubleValue is safe to call
            let value = unsafe {
                let slider: &NSSlider = &*sender;
                slider.doubleValue()
            };
            TranscriptionWindow::set_transparency(value);
            SettingsWindow::update_transparency_label(value);

            // Persist the preference
            if let Err(e) = preferences::set_overlay_transparency(value) {
                error!("Failed to save overlay transparency preference: {}", e);
            }
        }

        /// Handle background segmented control selection
        #[method(handleBackgroundSegment:)]
        fn handle_background_segment(&self, sender: *mut NSSegmentedControl) {
            // SAFETY: sender is a valid NSSegmentedControl passed by AppKit, selectedSegment is safe
            let selected = unsafe {
                let control: &NSSegmentedControl = &*sender;
                control.selectedSegment()
            };
            // 0 = Dark, 1 = Light
            let is_dark = selected == 0;
            TranscriptionWindow::set_dark_mode(is_dark);

            // Persist the preference
            if let Err(e) = preferences::set_is_dark_mode(is_dark) {
                error!("Failed to save dark mode preference: {}", e);
            }
        }

        #[method(handleChooseLocation:)]
        fn handle_choose_location(&self, _sender: *mut NSObject) {
            SettingsWindow::show_folder_picker();
        }

        #[method(handleResetLocation:)]
        fn handle_reset_location(&self, _sender: *mut NSObject) {
            SettingsWindow::reset_transcript_location();
        }

        #[method(handleChooseScreenshotLocation:)]
        fn handle_choose_screenshot_location(&self, _sender: *mut NSObject) {
            SettingsWindow::show_screenshot_folder_picker();
        }

        #[method(handleResetScreenshotLocation:)]
        fn handle_reset_screenshot_location(&self, _sender: *mut NSObject) {
            SettingsWindow::reset_screenshot_location();
        }

        /// Handle save Azure credentials button click
        #[method(handleSaveAzureCredentials:)]
        fn handle_save_azure_credentials(&self, _sender: *mut NSObject) {
            SettingsWindow::save_azure_credentials();
        }

        /// Handle clear Azure credentials button click
        #[method(handleClearAzureCredentials:)]
        fn handle_clear_azure_credentials(&self, _sender: *mut NSObject) {
            SettingsWindow::clear_azure_credentials();
        }

        /// Handle save OpenAI credentials button click
        #[method(handleSaveOpenAICredentials:)]
        fn handle_save_openai_credentials(&self, _sender: *mut NSObject) {
            SettingsWindow::save_openai_credentials();
        }

        /// Handle clear OpenAI credentials button click
        #[method(handleClearOpenAICredentials:)]
        fn handle_clear_openai_credentials(&self, _sender: *mut NSObject) {
            SettingsWindow::clear_openai_credentials();
        }

        /// Handle AI provider segmented control selection
        #[method(handleProviderChanged:)]
        fn handle_provider_changed(&self, sender: *mut NSSegmentedControl) {
            // SAFETY: sender is a valid NSSegmentedControl passed by AppKit
            let selected = unsafe {
                let control: &NSSegmentedControl = &*sender;
                control.selectedSegment()
            };
            // 0 = Azure OpenAI, 1 = OpenAI
            SettingsWindow::handle_provider_selection(selected);
        }
    }

    unsafe impl NSObjectProtocol for SettingsActionDelegate {}
);

impl SettingsActionDelegate {
    /// Create a new settings action delegate.
    ///
    /// Must be called on the main thread.
    pub(crate) fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let alloc = mtm.alloc::<Self>();
        // SAFETY: NSObject's init is safe to call on an allocated instance
        unsafe { msg_send_id![alloc, init] }
    }
}
