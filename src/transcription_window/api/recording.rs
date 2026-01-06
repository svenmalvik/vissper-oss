//! Recording and processing state management for the transcription window

use block2::RcBlock;
use objc2::msg_send;
use objc2_app_kit::NSColor;
use objc2_foundation::NSString;
use std::sync::atomic::Ordering;
use tracing::error;

use super::dispatch_to_main;
use crate::transcription_window::state::{IS_RECORDING, TRANSCRIPTION_WINDOW};

/// Set the recording state indicator.
///
/// When `recording` is true, shows a red indicator with "Recording" text.
/// When false, hides the indicator entirely.
pub(crate) fn set_recording_state(recording: bool) {
    // Track recording state globally for tab behavior
    IS_RECORDING.store(recording, Ordering::SeqCst);

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in set_recording_state");
            return;
        };

        // SAFETY: msg_send calls to valid NSView and NSTextField objects
        unsafe {
            if recording {
                let red_color = NSColor::colorWithRed_green_blue_alpha(0.9, 0.2, 0.2, 1.0);
                let _: () = msg_send![&inner.recording_indicator, setBackgroundColor: &*red_color];

                let text_color = NSColor::colorWithRed_green_blue_alpha(0.9, 0.3, 0.3, 1.0);
                inner.recording_label.setTextColor(Some(&text_color));
                inner
                    .recording_label
                    .setStringValue(&NSString::from_str("Recording"));

                let _: () = msg_send![&inner.recording_indicator, setHidden: false];
                let _: () = msg_send![&inner.recording_label, setHidden: false];
            } else {
                let _: () = msg_send![&inner.recording_indicator, setHidden: true];
                let _: () = msg_send![&inner.recording_label, setHidden: true];
            }
        }
    });

    dispatch_to_main(&block);
}

/// Set the recording type label in the header to "Transcription".
///
/// Updates the header label to indicate live transcription mode.
pub(crate) fn set_recording_type() {
    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in set_recording_type");
            return;
        };

        // SAFETY: setStringValue is safe on valid NSTextField
        unsafe {
            inner
                .recording_type_label
                .setStringValue(&NSString::from_str("Live Transcription"));
        }
    });

    dispatch_to_main(&block);
}

/// Set the processing state indicator.
///
/// When `processing` is true, shows an orange indicator with "Processing" text.
/// When false, hides the indicator entirely.
pub(crate) fn set_processing_state(processing: bool) {
    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in set_processing_state");
            return;
        };

        // SAFETY: msg_send calls to valid NSView and NSTextField objects
        unsafe {
            if processing {
                let orange_color = NSColor::colorWithRed_green_blue_alpha(0.95, 0.6, 0.1, 1.0);
                let _: () =
                    msg_send![&inner.recording_indicator, setBackgroundColor: &*orange_color];

                let text_color = NSColor::colorWithRed_green_blue_alpha(0.95, 0.6, 0.1, 1.0);
                inner.recording_label.setTextColor(Some(&text_color));
                inner
                    .recording_label
                    .setStringValue(&NSString::from_str("Processing"));

                let _: () = msg_send![&inner.recording_indicator, setHidden: false];
                let _: () = msg_send![&inner.recording_label, setHidden: false];
            } else {
                let _: () = msg_send![&inner.recording_indicator, setHidden: true];
                let _: () = msg_send![&inner.recording_label, setHidden: true];
            }
        }
    });

    dispatch_to_main(&block);
}
