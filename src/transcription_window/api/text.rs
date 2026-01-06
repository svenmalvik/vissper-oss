//! Text display operations for the transcription window

use block2::RcBlock;
use objc2::msg_send;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSScrollView, NSTextView};
use objc2_foundation::{NSAttributedString, NSRange, NSRect};
use std::sync::atomic::Ordering;
use tracing::error;

use super::dispatch_to_main;
use crate::transcription_window::markdown::create_attributed_string;
use crate::transcription_window::state::{TabType, IS_DARK_MODE, TRANSCRIPTION_WINDOW};

/// Update the displayed transcription text with markdown rendering.
///
/// Shows committed segments plus an optional partial transcript.
/// Updates the currently active tab's text view and auto-scrolls
/// to the bottom if the user was already near the bottom.
#[allow(dead_code)]
pub(crate) fn update_text(committed: &str, partial: Option<&str>, use_monospaced: bool) {
    let committed = committed.to_string();
    let partial = partial.map(|s| s.to_string());
    let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(mut inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in update_text");
            return;
        };

        // Get the active tab first
        let active_tab = inner.active_tab;

        // Build display text: committed text + partial (if any)
        let display_text = match partial.as_deref() {
            Some(p) if !p.is_empty() => {
                if committed.is_empty() {
                    p.to_string()
                } else {
                    format!("{} {}", committed, p)
                }
            }
            _ => committed.clone(),
        };

        // Store in tab content if this is the live tab
        if active_tab == TabType::Live {
            inner.tab_content.live_transcript = display_text.clone();
        }

        // Add padding at the end
        let padded_text = format!("{}\n\n\n\n\n\n", display_text);

        // Create attributed string with markdown parsing
        let attr_string = create_attributed_string(&padded_text, is_dark, use_monospaced);

        // Determine which views to use and check scroll position
        let should_scroll = match active_tab {
            TabType::Live => check_scroll_position_for_view(&inner.live_scroll_view),
            TabType::BasicPolish => check_scroll_position_for_view(&inner.polished_scroll_view),
            TabType::MeetingNotes => check_scroll_position_for_view(&inner.meeting_scroll_view),
        };

        // Update the appropriate text view
        match active_tab {
            TabType::Live => {
                set_text_view_attributed_string(&inner.live_text_view, &attr_string);
                if should_scroll {
                    scroll_to_bottom_for_view(&inner.live_text_view);
                }
            }
            TabType::BasicPolish => {
                set_text_view_attributed_string(&inner.polished_text_view, &attr_string);
                if should_scroll {
                    scroll_to_bottom_for_view(&inner.polished_text_view);
                }
            }
            TabType::MeetingNotes => {
                set_text_view_attributed_string(&inner.meeting_text_view, &attr_string);
                if should_scroll {
                    scroll_to_bottom_for_view(&inner.meeting_text_view);
                }
            }
        }
    });

    dispatch_to_main(&block);
}

/// Clear the transcription text (clears the live tab).
///
/// Resets the live tab to show "Listening..." placeholder text.
#[allow(dead_code)]
pub(crate) fn clear() {
    let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(mut inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in clear");
            return;
        };

        let attr_string = create_attributed_string("Listening...\n\n\n\n\n\n", is_dark, true);
        set_text_view_attributed_string(&inner.live_text_view, &attr_string);
        // Clear stored content
        inner.tab_content.live_transcript.clear();
    });

    dispatch_to_main(&block);
}

/// Update live transcript content (Tab 1) - used during recording.
///
/// Combines committed and partial text, stores it in the tab content,
/// and updates the display with auto-scroll behavior.
pub(crate) fn update_live_text(committed: &str, partial: Option<&str>) {
    let committed = committed.to_string();
    let partial = partial.map(|s| s.to_string());
    let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(mut inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in update_live_text");
            return;
        };

        let should_scroll_to_bottom = check_scroll_position_for_view(&inner.live_scroll_view);

        // Build display text: committed text + partial (if any)
        let display_text = match partial.as_deref() {
            Some(p) if !p.is_empty() => {
                if committed.is_empty() {
                    p.to_string()
                } else {
                    format!("{} {}", committed, p)
                }
            }
            _ => committed.clone(),
        };

        // Store the raw transcript
        inner.tab_content.live_transcript = display_text.clone();

        // Add padding at the end
        let display_text = format!("{}\n\n\n\n\n\n", display_text);

        // Create attributed string with markdown parsing (monospaced for live)
        let attr_string = create_attributed_string(&display_text, is_dark, true);

        // Update live text view
        set_text_view_attributed_string(&inner.live_text_view, &attr_string);

        // Scroll to bottom if we're on the live tab and near bottom
        if inner.active_tab == TabType::Live && should_scroll_to_bottom {
            scroll_to_bottom_for_view(&inner.live_text_view);
        }
    });

    dispatch_to_main(&block);
}

/// Check if scroll view is at or near the bottom.
///
/// Returns `true` if the visible area is within 50 points of the bottom,
/// indicating the user wants to follow new content.
pub(super) fn check_scroll_position_for_view(scroll_view: &NSScrollView) -> bool {
    // SAFETY: msg_send calls to valid NSScrollView methods
    unsafe {
        let clip_view: *mut AnyObject = msg_send![scroll_view, contentView];
        if clip_view.is_null() {
            return true;
        }

        let document_view: *mut AnyObject = msg_send![scroll_view, documentView];
        if document_view.is_null() {
            return true;
        }

        let visible_rect: NSRect = msg_send![clip_view, documentVisibleRect];
        let doc_frame: NSRect = msg_send![document_view, frame];

        let bottom_of_visible = visible_rect.origin.y + visible_rect.size.height;
        let threshold = 50.0;
        bottom_of_visible >= doc_frame.size.height - threshold
    }
}

/// Scroll text view to the bottom.
///
/// Scrolls to make the last character visible.
pub(super) fn scroll_to_bottom_for_view(text_view: &NSTextView) {
    // SAFETY: msg_send calls to valid NSTextView methods
    unsafe {
        let text_storage: *mut AnyObject = msg_send![text_view, textStorage];
        if !text_storage.is_null() {
            let length: usize = msg_send![text_storage, length];
            if length > 0 {
                let end_range = NSRange::new(length.saturating_sub(1), 1);
                let _: () = msg_send![text_view, scrollRangeToVisible: end_range];
            }
        }
    }
}

/// Set attributed string content on a text view.
///
/// Updates the text storage of the given text view with the provided
/// attributed string content.
///
/// # Safety
/// This function uses unsafe Objective-C FFI. The text_view must be
/// a valid NSTextView with a valid textStorage.
pub(super) fn set_text_view_attributed_string(
    text_view: &NSTextView,
    attr_string: &NSAttributedString,
) {
    // SAFETY: msg_send to valid NSTextView's textStorage
    unsafe {
        let text_storage: *mut AnyObject = msg_send![text_view, textStorage];
        if !text_storage.is_null() {
            let _: () = msg_send![text_storage, setAttributedString: attr_string];
        }
    }
}
