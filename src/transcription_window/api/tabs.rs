//! Tab switching and state management for the transcription window

use block2::RcBlock;
use objc2::msg_send;
use objc2_foundation::NSString;
use std::sync::atomic::Ordering;
use tracing::{error, info};

use super::dispatch_to_main;
use super::recording::set_processing_state;
use super::text::set_text_view_attributed_string;
use crate::transcription_window::markdown::create_attributed_string;
use crate::transcription_window::state::{
    TabType, IS_DARK_MODE, IS_RECORDING, TRANSCRIPTION_WINDOW, WINDOW_CALLBACKS,
};

/// Handle tab change from segmented control.
///
/// Called when the user clicks on a tab in the segmented control.
/// If recording is active, shows a message to stop recording first.
/// Otherwise, switches to the selected tab and triggers on-demand
/// content generation if needed.
pub(crate) fn handle_tab_change(selected_index: isize) {
    let tab = TabType::from_index(selected_index);
    info!("Tab changed to: {:?}", tab);

    // Check if recording is active
    let is_recording = IS_RECORDING.load(Ordering::SeqCst);

    // If recording and user clicks on polished or meeting notes tab, show message
    if is_recording && tab != TabType::Live {
        show_stop_recording_message(tab);
        return;
    }

    // Check if we need to generate content on-demand
    let (needs_generation, transcript) = {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in handle_tab_change");
            return;
        };

        let needs_gen = match tab {
            TabType::Live => false,
            TabType::BasicPolish => inner.tab_content.polished_content.is_none(),
            TabType::MeetingNotes => inner.tab_content.meeting_notes_content.is_none(),
        };

        (needs_gen, inner.tab_content.live_transcript.clone())
    };

    // Switch to the tab first
    switch_to_tab(tab);

    if needs_generation {
        // If there's transcript content, trigger on-demand generation
        if !transcript.trim().is_empty() {
            trigger_on_demand_generation(tab, transcript);
        } else {
            // No transcript to polish, show prompt
            show_generate_prompt(tab);
        }
    }
}

/// Trigger on-demand generation for a specific tab.
///
/// Shows a "Generating..." message and calls the appropriate callback
/// to generate polished content or meeting notes.
fn trigger_on_demand_generation(tab: TabType, transcript: String) {
    let Some(callbacks) = WINDOW_CALLBACKS.get() else {
        info!("No callbacks registered for on-demand generation");
        return;
    };

    // Show processing state
    set_processing_state(true);

    // Update the tab with "Generating..." message
    let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);
    let message = match tab {
        TabType::Live => return,
        TabType::BasicPolish => "⏳ Generating polished transcript...\n\n\n\n\n\n",
        TabType::MeetingNotes => "⏳ Generating meeting notes...\n\n\n\n\n\n",
    };

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in trigger_on_demand_generation");
            return;
        };

        let attr_string = create_attributed_string(message, is_dark, true);
        let text_view = match tab {
            TabType::Live => return,
            TabType::BasicPolish => &inner.polished_text_view,
            TabType::MeetingNotes => &inner.meeting_text_view,
        };

        set_text_view_attributed_string(text_view, &attr_string);
    });

    dispatch_to_main(&block);

    // Trigger the appropriate callback
    match tab {
        TabType::Live => {}
        TabType::BasicPolish => {
            info!("Triggering on-demand basic polishing");
            (callbacks.on_request_basic_polish)(transcript);
        }
        TabType::MeetingNotes => {
            info!("Triggering on-demand meeting notes generation");
            (callbacks.on_request_meeting_notes)(transcript);
        }
    }
}

/// Switch to a specific tab.
///
/// Updates the segmented control selection, shows/hides the appropriate
/// scroll views, and updates the header label.
pub(crate) fn switch_to_tab(tab: TabType) {
    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(mut inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in switch_to_tab");
            return;
        };

        // Update active tab
        inner.active_tab = tab;

        // Update segmented control selection
        // SAFETY: msg_send to valid NSSegmentedControl stored as NSView
        unsafe {
            let _: () = msg_send![&inner.segmented_control, setSelectedSegment: tab.to_index()];
        }

        // Show/hide scroll views based on active tab
        // SAFETY: msg_send setHidden: to valid NSScrollView objects
        unsafe {
            let _: () = msg_send![&inner.live_scroll_view, setHidden: tab != TabType::Live];
            let _: () =
                msg_send![&inner.polished_scroll_view, setHidden: tab != TabType::BasicPolish];
            let _: () =
                msg_send![&inner.meeting_scroll_view, setHidden: tab != TabType::MeetingNotes];
        }

        // Update header label based on tab
        let label_text = match tab {
            TabType::Live => "Live Transcription",
            TabType::BasicPolish => "Polished Transcript",
            TabType::MeetingNotes => "Meeting Notes",
        };
        // SAFETY: setStringValue is safe on valid NSTextField
        unsafe {
            inner
                .recording_type_label
                .setStringValue(&NSString::from_str(label_text));
        }
    });

    dispatch_to_main(&block);
}

/// Show "Stop recording first" message when user tries to access other tabs during recording.
fn show_stop_recording_message(tab: TabType) {
    let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);
    let message = "⚠️ Stop recording first to generate content for this tab.\n\n\n\n\n\n";

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(mut inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in show_stop_recording_message");
            return;
        };

        // Update active tab
        inner.active_tab = tab;

        // Update segmented control
        // SAFETY: msg_send to valid NSSegmentedControl stored as NSView
        unsafe {
            let _: () = msg_send![&inner.segmented_control, setSelectedSegment: tab.to_index()];
        }

        // Show/hide scroll views
        // SAFETY: msg_send setHidden: to valid NSScrollView objects
        unsafe {
            let _: () = msg_send![&inner.live_scroll_view, setHidden: tab != TabType::Live];
            let _: () =
                msg_send![&inner.polished_scroll_view, setHidden: tab != TabType::BasicPolish];
            let _: () =
                msg_send![&inner.meeting_scroll_view, setHidden: tab != TabType::MeetingNotes];
        }

        // Set the message in the appropriate text view
        let attr_string = create_attributed_string(message, is_dark, true);
        let text_view = match tab {
            TabType::Live => &inner.live_text_view,
            TabType::BasicPolish => &inner.polished_text_view,
            TabType::MeetingNotes => &inner.meeting_text_view,
        };

        set_text_view_attributed_string(text_view, &attr_string);
    });

    dispatch_to_main(&block);
}

/// Show prompt to generate content when tab has no content.
fn show_generate_prompt(tab: TabType) {
    let is_dark = IS_DARK_MODE.load(Ordering::SeqCst);
    let message = match tab {
        TabType::Live => return, // Live tab doesn't need generation
        TabType::BasicPolish => "📝 No polished content yet.\n\nThe transcript will be polished when you stop recording with 'Basic Polishing',\nor you can click here after recording to generate it.\n\n\n\n\n\n",
        TabType::MeetingNotes => "📋 No meeting notes yet.\n\nMeeting notes will be generated when you stop recording with 'Meeting Notes',\nor you can click here after recording to generate them.\n\n\n\n\n\n",
    };

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in show_generate_prompt");
            return;
        };

        let attr_string = create_attributed_string(message, is_dark, true);
        let text_view = match tab {
            TabType::Live => &inner.live_text_view,
            TabType::BasicPolish => &inner.polished_text_view,
            TabType::MeetingNotes => &inner.meeting_text_view,
        };

        set_text_view_attributed_string(text_view, &attr_string);
    });

    dispatch_to_main(&block);
}
