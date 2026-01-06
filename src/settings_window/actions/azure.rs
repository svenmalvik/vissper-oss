//! Azure credential management actions.

use objc2_foundation::NSString;
use tracing::{error, info};
use zeroize::Zeroize;

use crate::{keychain, menubar};

use super::super::SETTINGS_WINDOW;

/// Save Azure credentials from the UI fields to keychain.
pub(in crate::settings_window) fn save_azure_credentials() {
    // Extract values from UI while holding lock, then release lock before updating status
    let (endpoint_url, stt_deployment, polish_deployment, mut api_key) = {
        let Some(inner_cell) = SETTINGS_WINDOW.get() else {
            return;
        };
        let Ok(inner) = inner_cell.lock() else {
            return;
        };

        let endpoint = unsafe { inner.azure_endpoint_field.stringValue().to_string() };
        let stt = unsafe { inner.azure_stt_deployment_field.stringValue().to_string() };
        let polish = unsafe {
            inner
                .azure_polish_deployment_field
                .stringValue()
                .to_string()
        };
        let key = unsafe { inner.azure_api_key_field.stringValue().to_string() };

        (endpoint, stt, polish, key)
    }; // Lock released here

    // Validate inputs
    if endpoint_url.is_empty()
        || stt_deployment.is_empty()
        || polish_deployment.is_empty()
        || api_key.is_empty()
    {
        error!("Cannot save Azure credentials: all fields are required");
        update_azure_status("Status: Please fill all fields");
        api_key.zeroize();
        return;
    }

    // Store in keychain
    let creds = keychain::AzureCredentials {
        api_key: api_key.clone(),
        endpoint_url,
        stt_deployment,
        polish_deployment,
    };
    api_key.zeroize();

    match keychain::store_azure_credentials(&creds) {
        Ok(()) => {
            info!("Azure credentials saved to keychain");
            update_azure_status("Status: Credentials saved ✓");
            // Update menu bar to enable recording
            menubar::MenuBar::set_azure_credentials(true);
            // Clear the API key field after saving
            if let Some(inner_cell) = SETTINGS_WINDOW.get() {
                if let Ok(inner) = inner_cell.lock() {
                    unsafe {
                        inner
                            .azure_api_key_field
                            .setStringValue(&NSString::from_str("(stored in keychain)"));
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to save Azure credentials: {}", e);
            update_azure_status("Status: Failed to save");
        }
    }
}

/// Clear Azure credentials from keychain.
pub(in crate::settings_window) fn clear_azure_credentials() {
    match keychain::delete_azure_credentials() {
        Ok(()) => {
            info!("Azure credentials cleared from keychain");
            update_azure_status("Status: Credentials cleared");
            // Update menu bar to disable recording
            menubar::MenuBar::set_azure_credentials(false);
        }
        Err(e) => {
            error!("Failed to clear Azure credentials: {}", e);
            update_azure_status("Status: No credentials to clear");
        }
    }
}

/// Update the Azure status label.
pub(in crate::settings_window) fn update_azure_status(status: &str) {
    if let Some(inner) = SETTINGS_WINDOW.get() {
        if let Ok(inner) = inner.lock() {
            unsafe {
                inner
                    .azure_status_label
                    .setStringValue(&NSString::from_str(status));
            }
        }
    }
}
