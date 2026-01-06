//! Public API methods for the transcription window
//!
//! This module provides the public interface for controlling the transcription window,
//! organized into submodules by functionality.

mod pdf_writer;
mod recording;
mod save;
mod tab_content;
mod tabs;
mod text;
mod window;

use block2::RcBlock;
use objc2::msg_send;
use objc2_foundation::NSOperationQueue;

// Re-export all public functions from submodules
pub(crate) use recording::{set_processing_state, set_recording_state, set_recording_type};
pub(crate) use save::{handle_save_file_action, hide_save_button, show_save_button};
pub(crate) use tab_content::{
    get_live_transcript, reset_tabs, set_meeting_notes_content, set_polished_content,
};
pub(crate) use tabs::{handle_tab_change, switch_to_tab};
pub(crate) use text::{clear, update_live_text, update_text};
pub(crate) use window::{
    adjust_transparency, get_transparency, handle_hide_action, hide, is_dark_mode, set_dark_mode,
    set_transparency,
};

/// Dispatch a block to the main queue for UI operations.
///
/// # Safety
/// The block must be safe to execute on the main thread.
pub(super) fn dispatch_to_main(block: &RcBlock<dyn Fn()>) {
    // SAFETY: NSOperationQueue::mainQueue() returns a valid queue,
    // and addOperationWithBlock: safely schedules the block for execution.
    unsafe {
        let queue = NSOperationQueue::mainQueue();
        let _: () = msg_send![&queue, addOperationWithBlock: &**block];
    }
}
