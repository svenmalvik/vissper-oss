//! Global state and types for the transcription window module

use objc2::rc::Retained;
use objc2_app_kit::{NSScrollView, NSTextField, NSTextView, NSView, NSWindow};
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tracing::info;

use super::delegates::{HoverButton, WindowActionDelegate};
use crate::preferences;

/// Tab types for the transcription window
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TabType {
    #[default]
    Live,
    BasicPolish,
    MeetingNotes,
}

impl TabType {
    /// Convert from segment index to TabType
    pub fn from_index(index: isize) -> Self {
        match index {
            0 => TabType::Live,
            1 => TabType::BasicPolish,
            2 => TabType::MeetingNotes,
            _ => TabType::Live,
        }
    }

    /// Convert to segment index
    pub fn to_index(self) -> isize {
        match self {
            TabType::Live => 0,
            TabType::BasicPolish => 1,
            TabType::MeetingNotes => 2,
        }
    }
}

/// Content storage for each tab
#[derive(Debug, Clone, Default)]
pub(super) struct TabContent {
    /// Raw live transcript (always preserved)
    pub live_transcript: String,
    /// Basic polished content (None if not yet generated)
    pub polished_content: Option<String>,
    /// Meeting notes content (None if not yet generated)
    pub meeting_notes_content: Option<String>,
}

/// Global state for the transcription window
pub(super) static TRANSCRIPTION_WINDOW: OnceCell<Mutex<TranscriptionWindowInner>> = OnceCell::new();

/// Global callbacks for window actions
pub(super) static WINDOW_CALLBACKS: OnceCell<WindowCallbacks> = OnceCell::new();

/// Global state for current transparency (0.95 * 100 = 95)
pub(super) static CURRENT_TRANSPARENCY: AtomicU32 = AtomicU32::new(95);

/// Global state for dark/light mode (true = dark, false = light)
pub(super) static IS_DARK_MODE: AtomicBool = AtomicBool::new(true);

/// Global state for recording status (true = actively recording)
pub(super) static IS_RECORDING: AtomicBool = AtomicBool::new(false);

/// Global state for pending transcript (to be saved when user clicks Save button)
pub(super) static PENDING_TRANSCRIPT: OnceCell<RwLock<Option<String>>> = OnceCell::new();

/// Initialize or get the pending transcript storage
pub(super) fn pending_transcript_storage() -> &'static RwLock<Option<String>> {
    PENDING_TRANSCRIPT.get_or_init(|| RwLock::new(None))
}

/// Callbacks for window actions
pub(crate) struct WindowCallbacks {
    pub(crate) on_hide: Arc<dyn Fn() + Send + Sync>,
    /// Callback to request basic polishing on-demand (takes raw transcript)
    pub(crate) on_request_basic_polish: Arc<dyn Fn(String) + Send + Sync>,
    /// Callback to request meeting notes on-demand (takes raw transcript)
    pub(crate) on_request_meeting_notes: Arc<dyn Fn(String) + Send + Sync>,
}

/// Inner transcription window state
#[allow(dead_code)]
pub(super) struct TranscriptionWindowInner {
    pub window: Retained<NSWindow>,
    // Tab control (NSSegmentedControl stored as NSView since objc2_app_kit doesn't export it)
    pub segmented_control: Retained<NSView>,
    pub active_tab: TabType,
    pub tab_content: TabContent,
    // Tab 1: Live transcription
    pub live_scroll_view: Retained<NSScrollView>,
    pub live_text_view: Retained<NSTextView>,
    // Tab 2: Basic polishing
    pub polished_scroll_view: Retained<NSScrollView>,
    pub polished_text_view: Retained<NSTextView>,
    // Tab 3: Meeting notes
    pub meeting_scroll_view: Retained<NSScrollView>,
    pub meeting_text_view: Retained<NSTextView>,
    // Header elements
    pub header_view: Retained<NSView>,
    pub hide_button: Retained<HoverButton>,
    pub recording_type_label: Retained<NSTextField>,
    // Recording indicator (center bottom)
    pub recording_indicator: Retained<NSView>,
    pub recording_label: Retained<NSTextField>,
    // Save button (center bottom, shown after recording to allow manual save)
    pub save_button: Retained<HoverButton>,
    // Delegate (kept alive)
    pub delegate: Retained<WindowActionDelegate>,
}

unsafe impl Send for TranscriptionWindowInner {}

/// Load appearance preferences from persistent storage and apply them to global state.
///
/// This should be called once at app startup before showing the transcription window.
pub(super) fn load_appearance_preferences() {
    let transparency = preferences::get_overlay_transparency();
    let transparency_int = (transparency * 100.0) as u32;
    CURRENT_TRANSPARENCY.store(transparency_int, Ordering::SeqCst);
    info!(
        "Loaded overlay transparency from preferences: {}%",
        transparency_int
    );

    let is_dark = preferences::get_is_dark_mode();
    IS_DARK_MODE.store(is_dark, Ordering::SeqCst);
    info!(
        "Loaded background mode from preferences: {}",
        if is_dark { "dark" } else { "light" }
    );
}
