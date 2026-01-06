//! AI provider selection actions.

use objc2::rc::Retained;
use objc2::sel;
use objc2_app_kit::{NSSegmentedControl, NSView};
use objc2_foundation::{CGFloat, MainThreadMarker, NSPoint, NSRect, NSSize};
use tracing::{error, info};

use crate::preferences::{self, AiProvider};
use crate::{keychain, menubar};

use super::super::{constants, controls, SettingsActionDelegate, SETTINGS_WINDOW};
use super::{update_azure_status, update_openai_status};

/// Create the AI provider selector control.
pub(in crate::settings_window) fn create_provider_selector(
    mtm: MainThreadMarker,
    content_view: &NSView,
    delegate: &SettingsActionDelegate,
) -> Retained<NSSegmentedControl> {
    let content_width = content_view.frame().size.width;
    let control_width: CGFloat = 220.0;
    let control_height: CGFloat = 28.0;
    let y_pos: CGFloat = 20.0; // Below Screenshot Location and separator

    // Create label
    let label_frame = NSRect::new(
        NSPoint::new(constants::PADDING, y_pos + 6.0),
        NSSize::new(100.0, 20.0),
    );
    let label = controls::create_section_label(mtm, label_frame, "AI Provider");

    // Create segmented control
    let control_x = content_width - control_width - constants::PADDING;
    let control_frame = NSRect::new(
        NSPoint::new(control_x, y_pos),
        NSSize::new(control_width, control_height),
    );

    // Determine initial selection based on saved preference
    let current_provider = preferences::get_ai_provider();
    let selected_segment = match current_provider {
        AiProvider::Azure => 0,
        AiProvider::OpenAI => 1,
    };

    let control = controls::create_segmented_control(
        mtm,
        control_frame,
        &["Azure OpenAI", "OpenAI"],
        selected_segment,
        delegate,
        sel!(handleProviderChanged:),
    );

    unsafe {
        content_view.addSubview(&label);
        content_view.addSubview(&control);
    }

    control
}

/// Handle AI provider selection change.
pub(in crate::settings_window) fn handle_provider_selection(selected_segment: isize) {
    let provider = if selected_segment == 0 {
        AiProvider::Azure
    } else {
        AiProvider::OpenAI
    };

    // Save the preference
    if let Err(e) = preferences::set_ai_provider(provider) {
        error!("Failed to save AI provider preference: {}", e);
        return;
    }

    info!("AI provider changed to: {}", provider);

    // Check if credentials exist for the selected provider
    let has_credentials = match provider {
        AiProvider::Azure => keychain::get_azure_credentials().is_ok(),
        AiProvider::OpenAI => keychain::get_openai_credentials().is_ok(),
    };

    // Update menu bar state
    menubar::MenuBar::set_azure_credentials(has_credentials);

    // If no credentials, switch to the appropriate tab and show warning
    if !has_credentials {
        // Dispatch asynchronously to avoid potential deadlock
        let tab_index: isize = match provider {
            AiProvider::Azure => 1,
            AiProvider::OpenAI => 2,
        };
        dispatch::Queue::main().exec_async(move || {
            if let Some(inner) = SETTINGS_WINDOW.get() {
                if let Ok(inner) = inner.lock() {
                    // Switch to the appropriate credentials tab
                    unsafe {
                        inner.tab_view.selectTabViewItemAtIndex(tab_index);
                    }
                }
            }

            // Update the status label with a warning
            let warning = "Please enter credentials to use this provider";
            match provider {
                AiProvider::Azure => {
                    update_azure_status(warning);
                }
                AiProvider::OpenAI => {
                    update_openai_status(warning);
                }
            }
        });
    }
}
