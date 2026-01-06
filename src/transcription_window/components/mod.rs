//! UI component creation functions for header, text view, and tab control
//!
//! This module re-exports component creation functions from submodules.

mod header;
mod tab_control;
mod text_view;

pub(in crate::transcription_window) use header::create_header;
pub(in crate::transcription_window) use tab_control::create_tab_control;
pub(in crate::transcription_window) use text_view::create_scrollable_text_view;
