//! OpenAI settings UI controls.
//!
//! Simplified version of Azure controls since OpenAI only requires an API key.

use objc2::rc::Retained;
use objc2::{msg_send, msg_send_id};
use objc2_app_kit::NSTextField;
use objc2_foundation::{CGFloat, MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

use super::helpers::{create_section_label, create_small_button};
use crate::keychain::OpenAICredentials;
use crate::settings_window::constants::PADDING;
use crate::settings_window::delegate::SettingsActionDelegate;

/// OpenAI controls returned to caller for state management.
pub(crate) struct OpenAIControls {
    /// API key field. Note: Uses NSTextField since objc2-app-kit doesn't export NSSecureTextField.
    /// The API key is stored securely in the macOS Keychain.
    pub(crate) api_key_field: Retained<NSTextField>,
    pub(crate) status_label: Retained<NSTextField>,
}

/// Add OpenAI connection controls to the settings window.
///
/// Creates a simple section with:
/// - API Key field
/// - Status label and save/clear buttons
///
/// If `saved_credentials` is provided, the API key field will show "(stored in keychain)".
pub(crate) fn add_openai_controls(
    mtm: MainThreadMarker,
    content_view: &objc2_app_kit::NSView,
    delegate: &SettingsActionDelegate,
    saved_credentials: Option<&OpenAICredentials>,
) -> OpenAIControls {
    // Get content view width for layout calculations
    let content_width = content_view.frame().size.width;

    let has_credentials = saved_credentials.is_some();
    let field_height: CGFloat = 22.0;
    let label_height: CGFloat = 16.0;
    let button_height: CGFloat = 28.0;

    // Section header
    let section_y: CGFloat = 280.0;
    let section_label_frame = NSRect::new(
        NSPoint::new(PADDING, section_y),
        NSSize::new(content_width - PADDING * 2.0, 20.0),
    );
    let section_label =
        create_section_label(mtm, section_label_frame, "OpenAI Credentials (Required)");

    // API Key field (centered, wider since it's the only field)
    let field_width = content_width - PADDING * 2.0;
    let field_x = PADDING;

    let key_label_y: CGFloat = 245.0;
    let key_field_y: CGFloat = 220.0;

    let key_label = create_field_label_at(mtm, field_x, key_label_y, field_width, "API Key");
    let api_key_field = create_text_field(
        mtm,
        NSRect::new(
            NSPoint::new(field_x, key_field_y),
            NSSize::new(field_width, field_height),
        ),
        if has_credentials {
            "(stored in keychain)"
        } else {
            "sk-..."
        },
    );

    // Helper text
    let helper_y: CGFloat = 185.0;
    let helper_label = create_helper_label_at(
        mtm,
        PADDING,
        helper_y,
        content_width - PADDING * 2.0,
        label_height * 2.0,
        "Get your API key from platform.openai.com. Uses gpt-4o-transcribe for transcription and gpt-5.2 for polishing.",
    );

    // Status label
    let status_y: CGFloat = 145.0;
    let status_text = if has_credentials {
        "Status: Credentials saved ✓"
    } else {
        "Status: Enter your OpenAI API key to enable recording"
    };
    let status_label = create_status_label_at(
        mtm,
        PADDING,
        status_y,
        content_width - PADDING * 2.0,
        label_height,
        status_text,
    );

    // Buttons
    let buttons_y: CGFloat = 105.0;
    let save_button_width: CGFloat = 120.0;
    let clear_button_width: CGFloat = 130.0;
    let buttons_total_width = save_button_width + clear_button_width + 10.0;
    let buttons_x = (content_width - buttons_total_width) / 2.0;

    let save_button = create_small_button(
        mtm,
        NSRect::new(
            NSPoint::new(buttons_x, buttons_y),
            NSSize::new(save_button_width, button_height),
        ),
        "Save Credentials",
        delegate,
        objc2::sel!(handleSaveOpenAICredentials:),
    );

    let clear_button = create_small_button(
        mtm,
        NSRect::new(
            NSPoint::new(buttons_x + save_button_width + 10.0, buttons_y),
            NSSize::new(clear_button_width, button_height),
        ),
        "Clear Credentials",
        delegate,
        objc2::sel!(handleClearOpenAICredentials:),
    );

    // Add all subviews
    unsafe {
        content_view.addSubview(&section_label);
        content_view.addSubview(&key_label);
        content_view.addSubview(&api_key_field);
        content_view.addSubview(&helper_label);
        content_view.addSubview(&status_label);
        content_view.addSubview(&save_button);
        content_view.addSubview(&clear_button);
    }

    OpenAIControls {
        api_key_field,
        status_label,
    }
}

/// Create a field label at a specific position.
fn create_field_label_at(
    mtm: MainThreadMarker,
    x: CGFloat,
    y: CGFloat,
    width: CGFloat,
    text: &str,
) -> Retained<NSTextField> {
    let frame = NSRect::new(NSPoint::new(x, y), NSSize::new(width, 16.0));

    let label: Retained<NSTextField> =
        unsafe { msg_send_id![mtm.alloc::<NSTextField>(), initWithFrame: frame] };

    unsafe {
        label.setEditable(false);
        label.setSelectable(false);
        label.setBordered(false);
        label.setDrawsBackground(false);
        label.setStringValue(&NSString::from_str(text));

        let font = objc2_app_kit::NSFont::systemFontOfSize(11.0);
        label.setFont(Some(&font));
    }

    label
}

/// Create an editable single-line text field with placeholder.
fn create_text_field(
    mtm: MainThreadMarker,
    frame: NSRect,
    placeholder: &str,
) -> Retained<NSTextField> {
    let field: Retained<NSTextField> =
        unsafe { msg_send_id![mtm.alloc::<NSTextField>(), initWithFrame: frame] };

    unsafe {
        field.setEditable(true);
        field.setSelectable(true);
        field.setBordered(true);
        field.setDrawsBackground(true);
        let _: () = msg_send![&field, setPlaceholderString: &*NSString::from_str(placeholder)];

        // Configure for single-line mode (no word wrap)
        let cell: *mut objc2::runtime::AnyObject = msg_send![&field, cell];
        if !cell.is_null() {
            // NSLineBreakByTruncatingTail = 4
            let _: () = msg_send![cell, setLineBreakMode: 4_usize];
            let _: () = msg_send![cell, setUsesSingleLineMode: true];
            let _: () = msg_send![cell, setScrollable: true];
        }

        let font = objc2_app_kit::NSFont::systemFontOfSize(12.0);
        field.setFont(Some(&font));
    }

    field
}

/// Create a helper text label at a specific position.
fn create_helper_label_at(
    mtm: MainThreadMarker,
    x: CGFloat,
    y: CGFloat,
    width: CGFloat,
    height: CGFloat,
    text: &str,
) -> Retained<NSTextField> {
    let frame = NSRect::new(NSPoint::new(x, y), NSSize::new(width, height));

    let label: Retained<NSTextField> =
        unsafe { msg_send_id![mtm.alloc::<NSTextField>(), initWithFrame: frame] };

    unsafe {
        label.setEditable(false);
        label.setSelectable(false);
        label.setBordered(false);
        label.setDrawsBackground(false);
        label.setStringValue(&NSString::from_str(text));

        let font = objc2_app_kit::NSFont::systemFontOfSize(10.0);
        label.setFont(Some(&font));

        // Set text color to gray for helper text
        let color = objc2_app_kit::NSColor::tertiaryLabelColor();
        label.setTextColor(Some(&color));
    }

    label
}

/// Create a status label at a specific position.
fn create_status_label_at(
    mtm: MainThreadMarker,
    x: CGFloat,
    y: CGFloat,
    width: CGFloat,
    height: CGFloat,
    text: &str,
) -> Retained<NSTextField> {
    let frame = NSRect::new(NSPoint::new(x, y), NSSize::new(width, height));

    let label: Retained<NSTextField> =
        unsafe { msg_send_id![mtm.alloc::<NSTextField>(), initWithFrame: frame] };

    unsafe {
        label.setEditable(false);
        label.setSelectable(false);
        label.setBordered(false);
        label.setDrawsBackground(false);
        label.setStringValue(&NSString::from_str(text));

        let font = objc2_app_kit::NSFont::systemFontOfSize(11.0);
        label.setFont(Some(&font));

        // Set text color to gray for status
        let color = objc2_app_kit::NSColor::secondaryLabelColor();
        label.setTextColor(Some(&color));
    }

    label
}
