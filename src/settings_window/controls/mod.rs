//! UI control builders for the settings window.
//!
//! Contains functions for creating and laying out UI elements
//! in the settings window sections.

mod azure;
mod background;
mod helpers;
mod location;
mod openai;
mod transparency;

pub(crate) use azure::{add_azure_controls, AzureControls};
pub(crate) use background::add_background_controls;
pub(crate) use helpers::{
    create_section_label, create_segmented_control, create_separator, create_tab_item,
    create_tab_view,
};
pub(crate) use location::{add_location_controls, add_screenshot_location_controls};
pub(crate) use openai::{add_openai_controls, OpenAIControls};
pub(crate) use transparency::add_transparency_controls;
