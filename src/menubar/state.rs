//! Application state and callbacks for menu bar

use std::sync::atomic::AtomicBool;

/// Application state shared between menu callbacks
#[derive(Debug)]
pub struct AppState {
    pub is_recording: AtomicBool,
    pub is_processing: AtomicBool,
    pub has_azure_credentials: AtomicBool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_recording: AtomicBool::new(false),
            is_processing: AtomicBool::new(false),
            has_azure_credentials: AtomicBool::new(false),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Callbacks for menu actions
pub struct MenuCallbacks {
    pub on_start_recording: Box<dyn Fn() + Send + Sync>,
    pub on_stop_no_polish: Box<dyn Fn() + Send + Sync>,
    pub on_stop_basic_polish: Box<dyn Fn() + Send + Sync>,
    pub on_stop_meeting_notes: Box<dyn Fn() + Send + Sync>,
    pub on_show_window: Box<dyn Fn() + Send + Sync>,
    pub on_screenshot: Box<dyn Fn() + Send + Sync>,
    pub on_region_screenshot: Box<dyn Fn() + Send + Sync>,
    pub on_settings: Box<dyn Fn() + Send + Sync>,
    pub on_quit: Box<dyn Fn() + Send + Sync>,
    pub on_update_available: Box<dyn Fn() + Send + Sync>,
}
