//! Settings window action handlers.
//!
//! This module contains handlers for user actions in the settings window,
//! extracted to keep the main mod.rs focused on window creation and state.

mod azure;
mod openai;
mod paths;
mod provider;

pub(super) use azure::{clear_azure_credentials, save_azure_credentials};
pub(super) use openai::{clear_openai_credentials, save_openai_credentials};
pub(super) use paths::{
    reset_screenshot_location, reset_transcript_location, show_folder_picker,
    show_screenshot_folder_picker,
};
pub(super) use provider::{create_provider_selector, handle_provider_selection};

// Re-export for use within action submodules
use azure::update_azure_status;
use openai::update_openai_status;
