//! Azure OpenAI settings UI controls.
//!
//! In the OSS version, Azure is the only option (no toggle).
//! Users must provide their own Azure OpenAI credentials.

use objc2::rc::Retained;
use objc2::{msg_send, msg_send_id};
use objc2_app_kit::NSTextField;
use objc2_foundation::{CGFloat, MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

use super::helpers::{create_section_label, create_small_button};
use crate::keychain::AzureCredentials;
use crate::settings_window::constants::PADDING;
use crate::settings_window::delegate::SettingsActionDelegate;

/// Azure controls returned to caller for state management.
pub(crate) struct AzureControls {
    pub(crate) endpoint_field: Retained<NSTextField>,
    pub(crate) stt_deployment_field: Retained<NSTextField>,
    pub(crate) polish_deployment_field: Retained<NSTextField>,
    /// API key field. Note: Uses NSTextField since objc2-app-kit doesn't export NSSecureTextField.
    /// The API key is stored securely in the macOS Keychain.
    pub(crate) api_key_field: Retained<NSTextField>,
    pub(crate) status_label: Retained<NSTextField>,
}

/// Add Azure OpenAI connection controls to the settings window.
///
/// Creates a section with two-column layout:
/// - Row 1: Endpoint URL | STT Deployment
/// - Row 2: Polish Deployment | API Key
/// - Status label and save/clear buttons
///
/// If `saved_credentials` is provided, the fields will be populated with saved values
/// (except API key which remains empty for security).
pub(crate) fn add_azure_controls(
    mtm: MainThreadMarker,
    content_view: &objc2_app_kit::NSView,
    delegate: &SettingsActionDelegate,
    saved_credentials: Option<&AzureCredentials>,
) -> AzureControls {
    // Get content view width for layout calculations
    let content_width = content_view.frame().size.width;

    let has_credentials = saved_credentials.is_some();
    let field_height: CGFloat = 22.0;
    let label_height: CGFloat = 16.0;
    let button_height: CGFloat = 28.0;
    let column_gap: CGFloat = 20.0;

    // Calculate column widths (two equal columns)
    let column_width = (content_width - PADDING * 2.0 - column_gap) / 2.0;
    let left_x = PADDING;
    let right_x = PADDING + column_width + column_gap;

    // Section header
    let section_y: CGFloat = 280.0;
    let section_label_frame = NSRect::new(
        NSPoint::new(PADDING, section_y),
        NSSize::new(content_width - PADDING * 2.0, 20.0),
    );
    let section_label = create_section_label(
        mtm,
        section_label_frame,
        "Azure OpenAI Credentials (Required)",
    );

    // Row 1: Endpoint URL (left) | STT Deployment (right)
    let row1_label_y: CGFloat = 245.0;
    let row1_field_y: CGFloat = 220.0;

    // Endpoint URL (left column)
    let endpoint_label =
        create_field_label_at(mtm, left_x, row1_label_y, column_width, "Endpoint URL");
    let endpoint_field = create_text_field(
        mtm,
        NSRect::new(
            NSPoint::new(left_x, row1_field_y),
            NSSize::new(column_width, field_height),
        ),
        "https://myresource.openai.azure.com",
    );
    if let Some(creds) = saved_credentials {
        unsafe {
            endpoint_field.setStringValue(&NSString::from_str(&creds.endpoint_url));
        }
    }

    // STT Deployment (right column)
    let stt_label =
        create_field_label_at(mtm, right_x, row1_label_y, column_width, "STT Deployment");
    let stt_deployment_field = create_text_field(
        mtm,
        NSRect::new(
            NSPoint::new(right_x, row1_field_y),
            NSSize::new(column_width, field_height),
        ),
        "gpt-4o-transcribe",
    );
    if let Some(creds) = saved_credentials {
        unsafe {
            stt_deployment_field.setStringValue(&NSString::from_str(&creds.stt_deployment));
        }
    }

    // Row 2: Polish Deployment (left) | API Key (right)
    let row2_label_y: CGFloat = 180.0;
    let row2_field_y: CGFloat = 155.0;

    // Polish Deployment (left column)
    let polish_label =
        create_field_label_at(mtm, left_x, row2_label_y, column_width, "Polish Deployment");
    let polish_deployment_field = create_text_field(
        mtm,
        NSRect::new(
            NSPoint::new(left_x, row2_field_y),
            NSSize::new(column_width, field_height),
        ),
        "gpt-4o",
    );
    if let Some(creds) = saved_credentials {
        unsafe {
            polish_deployment_field.setStringValue(&NSString::from_str(&creds.polish_deployment));
        }
    }

    // API Key (right column)
    let key_label = create_field_label_at(mtm, right_x, row2_label_y, column_width, "API Key");
    let api_key_field = create_text_field(
        mtm,
        NSRect::new(
            NSPoint::new(right_x, row2_field_y),
            NSSize::new(column_width, field_height),
        ),
        if has_credentials {
            "(stored in keychain)"
        } else {
            "Enter API key"
        },
    );

    // Status label
    let status_y: CGFloat = 115.0;
    let status_text = if has_credentials {
        "Status: Credentials saved ✓"
    } else {
        "Status: Enter your Azure OpenAI credentials to enable recording"
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
    let buttons_y: CGFloat = 75.0;
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
        objc2::sel!(handleSaveAzureCredentials:),
    );

    let clear_button = create_small_button(
        mtm,
        NSRect::new(
            NSPoint::new(buttons_x + save_button_width + 10.0, buttons_y),
            NSSize::new(clear_button_width, button_height),
        ),
        "Clear Credentials",
        delegate,
        objc2::sel!(handleClearAzureCredentials:),
    );

    // Add all subviews
    unsafe {
        content_view.addSubview(&section_label);
        content_view.addSubview(&endpoint_label);
        content_view.addSubview(&endpoint_field);
        content_view.addSubview(&stt_label);
        content_view.addSubview(&stt_deployment_field);
        content_view.addSubview(&polish_label);
        content_view.addSubview(&polish_deployment_field);
        content_view.addSubview(&key_label);
        content_view.addSubview(&api_key_field);
        content_view.addSubview(&status_label);
        content_view.addSubview(&save_button);
        content_view.addSubview(&clear_button);
    }

    AzureControls {
        endpoint_field,
        stt_deployment_field,
        polish_deployment_field,
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
