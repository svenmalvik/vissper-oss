//! State management for region selection.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::NSWindow;
use objc2_foundation::NSPoint;
use objc2_foundation::NSRect;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

use crate::recording::RecordingSession;

/// Window level above screenshot flash (1000) to appear on top
pub(super) const OVERLAY_WINDOW_LEVEL: isize = 1001;

/// Minimum selection size in points to avoid accidental clicks
pub(super) const MIN_SELECTION_SIZE: f64 = 5.0;

/// Global state for region selection
pub(super) static REGION_STATE: Lazy<Mutex<Option<RegionSelectionState>>> =
    Lazy::new(|| Mutex::new(None));

/// State tracking the selection overlay
pub(super) struct RegionSelectionState {
    pub(super) windows: Vec<Retained<NSWindow>>,
    pub(super) selection_origin: Option<NSPoint>,
    pub(super) current_rect: Option<NSRect>,
    pub(super) event_monitor: Option<Retained<AnyObject>>,
    pub(super) total_screen_height: f64,
    pub(super) recording_state: Arc<Mutex<Option<RecordingSession>>>,
}

// SAFETY: RegionSelectionState is only accessed from the main thread via MainThreadMarker checks.
unsafe impl Send for RegionSelectionState {}
