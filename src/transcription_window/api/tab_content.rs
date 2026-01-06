//! Tab content management for the transcription window

use block2::RcBlock;
use objc2::msg_send;
use objc2_foundation::NSString;
use std::sync::atomic::Ordering;
use tracing::error;

use super::dispatch_to_main;
use super::text::set_text_view_attributed_string;
use crate::transcription_window::markdown::create_attributed_string;
use crate::transcription_window::state::{TabType, IS_DARK_MODE, TRANSCRIPTION_WINDOW};

/// Set polished content (Tab 2).
///
/// Stores the content and updates the polished text view display.
pub(crate) fn set_polished_content(content: &str) {
    let content = content.to_string();
    let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(mut inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in set_polished_content");
            return;
        };

        // Store the polished content
        inner.tab_content.polished_content = Some(content.clone());

        // Add padding at the end
        let display_text = format!("{}\n\n\n\n\n\n", content);

        // Create attributed string (proportional font for polished)
        let attr_string = create_attributed_string(&display_text, is_dark, false);

        // Update polished text view
        set_text_view_attributed_string(&inner.polished_text_view, &attr_string);
    });

    dispatch_to_main(&block);
}

/// Set meeting notes content (Tab 3).
///
/// Stores the content and updates the meeting notes text view display.
pub(crate) fn set_meeting_notes_content(content: &str) {
    let content = content.to_string();
    let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(mut inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in set_meeting_notes_content");
            return;
        };

        // Store the meeting notes content
        inner.tab_content.meeting_notes_content = Some(content.clone());

        // Add padding at the end
        let display_text = format!("{}\n\n\n\n\n\n", content);

        // Create attributed string (proportional font for meeting notes)
        let attr_string = create_attributed_string(&display_text, is_dark, false);

        // Update meeting notes text view
        set_text_view_attributed_string(&inner.meeting_text_view, &attr_string);
    });

    dispatch_to_main(&block);
}

/// Get the current raw transcript for on-demand polishing.
///
/// Returns `None` if the window doesn't exist or the transcript is empty.
#[allow(dead_code)]
pub(crate) fn get_live_transcript() -> Option<String> {
    let inner = TRANSCRIPTION_WINDOW.get()?;
    let Ok(inner) = inner.lock() else {
        error!("Failed to acquire transcription window lock in get_live_transcript");
        return None;
    };

    let transcript = inner.tab_content.live_transcript.clone();
    if transcript.is_empty() {
        None
    } else {
        Some(transcript)
    }
}

/// Reset tab content when starting a new recording.
///
/// Clears all tab content, resets to the Live tab, and shows
/// placeholder text in all tabs.
pub(crate) fn reset_tabs() {
    let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(mut inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in reset_tabs");
            return;
        };

        // Reset tab content
        inner.tab_content.live_transcript.clear();
        inner.tab_content.polished_content = None;
        inner.tab_content.meeting_notes_content = None;
        inner.active_tab = TabType::Live;

        // Reset live tab text
        let live_attr = create_attributed_string("Listening...\n\n\n\n\n\n", is_dark, true);
        set_text_view_attributed_string(&inner.live_text_view, &live_attr);

        // Reset polished tab with placeholder
        let polished_attr = create_attributed_string(
            "Click to generate polished transcript...\n\n\n\n\n\n",
            is_dark,
            true,
        );
        set_text_view_attributed_string(&inner.polished_text_view, &polished_attr);

        // Reset meeting notes tab with placeholder
        let meeting_attr = create_attributed_string(
            "Click to generate meeting notes...\n\n\n\n\n\n",
            is_dark,
            true,
        );
        set_text_view_attributed_string(&inner.meeting_text_view, &meeting_attr);

        // Switch to live tab
        // SAFETY: msg_send to valid NSSegmentedControl and NSScrollView objects
        unsafe {
            let _: () = msg_send![&inner.segmented_control, setSelectedSegment: 0isize];
            let _: () = msg_send![&inner.live_scroll_view, setHidden: false];
            let _: () = msg_send![&inner.polished_scroll_view, setHidden: true];
            let _: () = msg_send![&inner.meeting_scroll_view, setHidden: true];
        }

        // Update header
        // SAFETY: setStringValue is safe on valid NSTextField
        unsafe {
            inner
                .recording_type_label
                .setStringValue(&NSString::from_str("Live Transcription"));
        }
    });

    dispatch_to_main(&block);
}
