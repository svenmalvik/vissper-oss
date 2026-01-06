//! OpenAI credential management actions.

use objc2_foundation::NSString;
use tracing::{error, info};
use zeroize::Zeroize;

use crate::preferences::{self, AiProvider};
use crate::{keychain, menubar};

use super::super::SETTINGS_WINDOW;

/// Save OpenAI credentials from the UI fields to keychain.
pub(in crate::settings_window) fn save_openai_credentials() {
    // Extract values from UI while holding lock
    let mut api_key = {
        let Some(inner_cell) = SETTINGS_WINDOW.get() else {
            return;
        };
        let Ok(inner) = inner_cell.lock() else {
            return;
        };

        unsafe { inner.openai_api_key_field.stringValue().to_string() }
    }; // Lock released here

    // Validate input
    if api_key.is_empty() || api_key == "(stored in keychain)" {
        error!("Cannot save OpenAI credentials: API key is required");
        update_openai_status("Status: Please enter your API key");
        api_key.zeroize();
        return;
    }

    // Store in keychain
    let creds = keychain::OpenAICredentials {
        api_key: api_key.clone(),
    };
    api_key.zeroize();

    match keychain::store_openai_credentials(&creds) {
        Ok(()) => {
            info!("OpenAI credentials saved to keychain");
            update_openai_status("Status: Credentials saved ✓");
            // Update menu bar if OpenAI is the selected provider
            if preferences::get_ai_provider() == AiProvider::OpenAI {
                menubar::MenuBar::set_azure_credentials(true);
            }
            // Clear the API key field after saving
            if let Some(inner_cell) = SETTINGS_WINDOW.get() {
                if let Ok(inner) = inner_cell.lock() {
                    unsafe {
                        inner
                            .openai_api_key_field
                            .setStringValue(&NSString::from_str("(stored in keychain)"));
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to save OpenAI credentials: {}", e);
            update_openai_status("Status: Failed to save");
        }
    }
}

/// Clear OpenAI credentials from keychain.
pub(in crate::settings_window) fn clear_openai_credentials() {
    match keychain::delete_openai_credentials() {
        Ok(()) => {
            info!("OpenAI credentials cleared from keychain");
            update_openai_status("Status: Credentials cleared");
            // Update menu bar if OpenAI is the selected provider
            if preferences::get_ai_provider() == AiProvider::OpenAI {
                menubar::MenuBar::set_azure_credentials(false);
            }
        }
        Err(e) => {
            error!("Failed to clear OpenAI credentials: {}", e);
            update_openai_status("Status: No credentials to clear");
        }
    }
}

/// Update the OpenAI status label.
pub(in crate::settings_window) fn update_openai_status(status: &str) {
    if let Some(inner) = SETTINGS_WINDOW.get() {
        if let Ok(inner) = inner.lock() {
            unsafe {
                inner
                    .openai_status_label
                    .setStringValue(&NSString::from_str(status));
            }
        }
    }
}
