//! File save operations for the transcription window

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use block2::RcBlock;
use chrono::Local;
use objc2::rc::Retained;
use objc2::{msg_send, msg_send_id, ClassType};
use objc2_app_kit::{NSPopUpButton, NSSavePanel, NSStackView, NSTextField};
use objc2_foundation::{CGRect, CGSize, MainThreadMarker, NSArray, NSPoint, NSString};
use tracing::{error, info};

use super::dispatch_to_main;
use super::pdf_writer;
use crate::storage;
use crate::transcription_window::state::{pending_transcript_storage, TRANSCRIPTION_WINDOW};

/// Modal response constant for OK button
const NS_MODAL_RESPONSE_OK: isize = 1;

/// Show the save button and store the transcript for later saving.
///
/// The transcript is stored in global state and can be saved when
/// the user clicks the save button.
pub(crate) fn show_save_button(transcript: String) {
    info!(
        "Showing save button for transcript ({} chars)",
        transcript.len()
    );

    // Store the transcript in global state
    if let Ok(mut stored_transcript) = pending_transcript_storage().write() {
        *stored_transcript = Some(transcript);
    } else {
        error!("Failed to store transcript for saving");
    }

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in show_save_button");
            return;
        };

        // SAFETY: msg_send setHidden: to valid NSButton
        unsafe {
            let _: () = msg_send![&inner.save_button, setHidden: false];
        }
    });

    dispatch_to_main(&block);
}

/// Hide the save button.
///
/// Also clears the stored transcript.
pub(crate) fn hide_save_button() {
    // Clear the stored transcript
    if let Ok(mut stored_transcript) = pending_transcript_storage().write() {
        *stored_transcript = None;
    }

    let block = RcBlock::new(move || {
        let Some(inner) = TRANSCRIPTION_WINDOW.get() else {
            return;
        };
        let Ok(inner) = inner.lock() else {
            error!("Failed to acquire transcription window lock in hide_save_button");
            return;
        };

        // SAFETY: msg_send setHidden: to valid NSButton
        unsafe {
            let _: () = msg_send![&inner.save_button, setHidden: true];
        }
    });

    dispatch_to_main(&block);
}

/// Handle save file button click.
///
/// Shows NSSavePanel for user to choose save location and saves
/// the transcript to the selected file.
pub(crate) fn handle_save_file_action() {
    info!("Save button clicked");

    // Get the stored transcript
    let transcript = {
        match pending_transcript_storage().read() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to read stored transcript: {}", e);
                None
            }
        }
    };

    let Some(transcript) = transcript else {
        error!("No transcript available to save");
        return;
    };

    // Must be on main thread for NSSavePanel
    let Some(mtm) = MainThreadMarker::new() else {
        error!("Not on main thread, cannot show save panel");
        return;
    };

    // Create and configure NSSavePanel
    // SAFETY: NSSavePanel::class() returns valid class, savePanel creates valid instance
    let panel: Retained<NSSavePanel> = unsafe { msg_send_id![NSSavePanel::class(), savePanel] };

    // Create format popup button for accessory view
    let format_popup = create_format_popup(mtm);

    // SAFETY: All msg_send calls are to valid NSSavePanel methods
    unsafe {
        // Generate default filename with timestamp (without extension - will be added based on format)
        let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S");
        let default_name = format!("transcript-{}", timestamp);
        panel.setNameFieldStringValue(&NSString::from_str(&default_name));

        // Set prompt and message
        panel.setPrompt(Some(&NSString::from_str("Save")));
        panel.setMessage(Some(&NSString::from_str(
            "Choose where to save the transcript",
        )));

        // Set accessory view with format dropdown
        let accessory_view = create_format_accessory_view(mtm, format_popup.clone());
        panel.setAccessoryView(Some(&accessory_view));

        // Allow all extensions (we'll enforce based on popup selection)
        let md_ext = NSString::from_str("md");
        let pdf_ext = NSString::from_str("pdf");
        let extensions: Retained<NSArray<NSString>> = NSArray::from_id_slice(&[md_ext, pdf_ext]);
        #[allow(deprecated)]
        panel.setAllowedFileTypes(Some(&extensions));

        // Set initial directory to user's preferred transcript location
        if let Some(transcript_dir) = storage::transcripts_dir() {
            // Ensure directory exists before setting it
            if !transcript_dir.exists() {
                let _ = std::fs::create_dir_all(&transcript_dir);
            }
            if transcript_dir.exists() {
                let url_string = format!("file://{}", transcript_dir.display());
                let ns_url_string = NSString::from_str(&url_string);
                let url: Option<Retained<objc2_foundation::NSURL>> =
                    msg_send_id![objc2_foundation::NSURL::class(), URLWithString: &*ns_url_string];
                if let Some(url) = url {
                    panel.setDirectoryURL(Some(&url));
                }
            }
        }
    }

    // Show panel and handle result
    // SAFETY: runModal is safe on valid NSSavePanel
    let response = unsafe { panel.runModal() };

    if response == NS_MODAL_RESPONSE_OK {
        // Get selected format from popup (0 = Markdown, 1 = PDF)
        let selected_index: isize = unsafe { msg_send![&format_popup, indexOfSelectedItem] };
        let extension = if selected_index == 1 { "pdf" } else { "md" };

        // SAFETY: URL() is safe on valid NSSavePanel after OK response
        if let Some(url) = unsafe { panel.URL() } {
            // SAFETY: path() is safe on valid NSURL
            if let Some(path_str) = unsafe { url.path() } {
                let mut path = PathBuf::from(path_str.to_string());

                // Ensure correct extension based on format selection
                path.set_extension(extension);

                // Write transcript to file (routes to PDF or text based on extension)
                match write_transcript_to_path(&path, &transcript) {
                    Ok(()) => {
                        info!("Transcript saved to: {:?}", path);
                        // Hide the save button after successful save
                        hide_save_button();
                    }
                    Err(e) => {
                        error!("Failed to save transcript: {}", e);
                    }
                }
            }
        }
    } else {
        info!("Save cancelled by user");
    }
}

/// Create the format selection popup button.
///
/// # Safety
/// Must be called from the main thread.
fn create_format_popup(mtm: MainThreadMarker) -> Retained<NSPopUpButton> {
    unsafe {
        let frame = CGRect::new(NSPoint::new(0.0, 0.0), CGSize::new(150.0, 25.0));
        let popup: Retained<NSPopUpButton> =
            msg_send_id![mtm.alloc::<NSPopUpButton>(), initWithFrame: frame, pullsDown: false];

        // Add format options
        let md_title = NSString::from_str("Markdown (.md)");
        let pdf_title = NSString::from_str("PDF (.pdf)");
        let _: () = msg_send![&popup, addItemWithTitle: &*md_title];
        let _: () = msg_send![&popup, addItemWithTitle: &*pdf_title];

        // Select Markdown by default
        let _: () = msg_send![&popup, selectItemAtIndex: 0_isize];

        popup
    }
}

/// Create the accessory view containing a label and the format popup.
///
/// # Safety
/// Must be called from the main thread.
fn create_format_accessory_view(
    mtm: MainThreadMarker,
    format_popup: Retained<NSPopUpButton>,
) -> Retained<NSStackView> {
    unsafe {
        // Create label
        let label_frame = CGRect::new(NSPoint::new(0.0, 0.0), CGSize::new(60.0, 25.0));
        let label: Retained<NSTextField> =
            msg_send_id![mtm.alloc::<NSTextField>(), initWithFrame: label_frame];
        let _: () = msg_send![&label, setStringValue: &*NSString::from_str("Format:")];
        let _: () = msg_send![&label, setBezeled: false];
        let _: () = msg_send![&label, setDrawsBackground: false];
        let _: () = msg_send![&label, setEditable: false];
        let _: () = msg_send![&label, setSelectable: false];

        // Create horizontal stack view with label and popup
        // NSTextField -> NSControl -> NSView
        let label_view: Retained<objc2_app_kit::NSView> =
            Retained::into_super(Retained::into_super(label));
        // NSPopUpButton -> NSButton -> NSControl -> NSView
        let popup_view: Retained<objc2_app_kit::NSView> =
            Retained::into_super(Retained::into_super(Retained::into_super(format_popup)));

        let views: Retained<NSArray<objc2_app_kit::NSView>> =
            NSArray::from_id_slice(&[label_view, popup_view]);

        let stack: Retained<NSStackView> =
            msg_send_id![NSStackView::class(), stackViewWithViews: &*views];

        let _: () = msg_send![&stack, setSpacing: 8.0_f64];

        // Set frame size for the stack view
        let stack_frame = CGRect::new(NSPoint::new(0.0, 0.0), CGSize::new(220.0, 32.0));
        let _: () = msg_send![&stack, setFrame: stack_frame];

        stack
    }
}

/// Write transcript to file, choosing format based on file extension.
///
/// Routes to PDF generation for `.pdf` files, or plain text for `.md`/`.txt`.
fn write_transcript_to_path(path: &Path, transcript: &str) -> Result<()> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("md");

    match extension.to_lowercase().as_str() {
        "pdf" => pdf_writer::write_pdf(path, transcript),
        _ => write_transcript_to_file(path, transcript)
            .with_context(|| format!("Failed to write transcript to {}", path.display())),
    }
}

/// Write transcript content to a text file.
fn write_transcript_to_file(path: &Path, transcript: &str) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create file {}", path.display()))?;
    file.write_all(transcript.as_bytes())
        .with_context(|| format!("Failed to write to {}", path.display()))?;
    file.flush()
        .with_context(|| format!("Failed to flush {}", path.display()))?;
    Ok(())
}
