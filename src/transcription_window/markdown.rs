//! Simple markdown parsing and rendering
//!
//! Converts markdown text to NSMutableAttributedString for display
//! in the transcription window with proper styling.

use objc2::rc::Retained;
use objc2::{msg_send, msg_send_id, ClassType};
use objc2_app_kit::{NSColor, NSFont};
use objc2_foundation::{NSMutableAttributedString, NSRange, NSString};

/// Simple markdown segment types for parsing markdown text.
///
/// Used for both UI rendering (NSAttributedString) and PDF export.
pub(crate) enum MarkdownSegment {
    Header1(String),     // # header
    Header2(String),     // ## header
    Header3(String),     // ### header
    BulletPoint(String), // - item or * item
    Bold(String),        // **text**
    Normal(String),      // regular text
}

/// Parse text into markdown segments (simple line-based parsing).
///
/// Supports headers (H1-H3), bullet points, bold text, and normal text.
pub(crate) fn parse_markdown(text: &str) -> Vec<MarkdownSegment> {
    let mut segments = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_start();

        if let Some(content) = trimmed.strip_prefix("### ") {
            segments.push(MarkdownSegment::Header3(content.to_string()));
            segments.push(MarkdownSegment::Normal("\n".to_string()));
        } else if let Some(content) = trimmed.strip_prefix("## ") {
            segments.push(MarkdownSegment::Header2(content.to_string()));
            segments.push(MarkdownSegment::Normal("\n".to_string()));
        } else if let Some(content) = trimmed.strip_prefix("# ") {
            segments.push(MarkdownSegment::Header1(content.to_string()));
            segments.push(MarkdownSegment::Normal("\n".to_string()));
        } else if let Some(content) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            segments.push(MarkdownSegment::BulletPoint(content.to_string()));
            segments.push(MarkdownSegment::Normal("\n".to_string()));
        } else if !line.is_empty() {
            parse_inline_formatting(line, &mut segments);
            segments.push(MarkdownSegment::Normal("\n".to_string()));
        } else {
            segments.push(MarkdownSegment::Normal("\n".to_string()));
        }
    }

    segments
}

/// Parse inline bold formatting within a line
fn parse_inline_formatting(text: &str, segments: &mut Vec<MarkdownSegment>) {
    let mut remaining = text;

    while !remaining.is_empty() {
        if let Some(start) = remaining.find("**") {
            if start > 0 {
                segments.push(MarkdownSegment::Normal(remaining[..start].to_string()));
            }

            let after_start = &remaining[start + 2..];
            if let Some(end) = after_start.find("**") {
                segments.push(MarkdownSegment::Bold(after_start[..end].to_string()));
                remaining = &after_start[end + 2..];
            } else {
                segments.push(MarkdownSegment::Normal(remaining.to_string()));
                break;
            }
        } else {
            segments.push(MarkdownSegment::Normal(remaining.to_string()));
            break;
        }
    }
}

/// Create an NSMutableAttributedString from markdown text with proper styling
pub(super) fn create_attributed_string(
    text: &str,
    is_dark: bool,
    use_monospaced: bool,
) -> Retained<NSMutableAttributedString> {
    let segments = parse_markdown(text);

    // Create empty mutable attributed string
    let result: Retained<NSMutableAttributedString> =
        unsafe { msg_send_id![NSMutableAttributedString::alloc(), init] };

    // Text color based on dark/light mode
    let text_color = if is_dark {
        unsafe { NSColor::whiteColor() }
    } else {
        unsafe { NSColor::blackColor() }
    };

    // Fonts
    let (regular_font, bold_font) = unsafe {
        if use_monospaced {
            // Use monospaced system font (SF Mono on modern macOS)
            // Weight 0.0 is Regular, 0.4 is Bold (approx)
            let regular: Retained<NSFont> = msg_send_id![
                NSFont::class(),
                monospacedSystemFontOfSize: 14.0,
                weight: 0.0
            ];
            let bold: Retained<NSFont> = msg_send_id![
                NSFont::class(),
                monospacedSystemFontOfSize: 14.0,
                weight: 0.4
            ];
            (regular, bold)
        } else {
            (
                NSFont::systemFontOfSize(14.0),
                NSFont::boldSystemFontOfSize(14.0),
            )
        }
    };

    let h1_font = unsafe { NSFont::boldSystemFontOfSize(20.0) };
    let h2_font = unsafe { NSFont::boldSystemFontOfSize(17.0) };
    let h3_font = unsafe { NSFont::boldSystemFontOfSize(15.0) };

    let color_attr = NSString::from_str("NSColor");
    let font_attr = NSString::from_str("NSFont");

    for segment in segments {
        let (text_content, font) = match &segment {
            MarkdownSegment::Header1(s) => (s.as_str(), &*h1_font),
            MarkdownSegment::Header2(s) => (s.as_str(), &*h2_font),
            MarkdownSegment::Header3(s) => (s.as_str(), &*h3_font),
            MarkdownSegment::BulletPoint(s) => {
                append_bullet_point(&result, s, &text_color, &regular_font);
                continue;
            }
            MarkdownSegment::Bold(s) => (s.as_str(), &*bold_font),
            MarkdownSegment::Normal(s) => (s.as_str(), &*regular_font),
        };

        append_styled_text(
            &result,
            text_content,
            font,
            &text_color,
            &color_attr,
            &font_attr,
        );
    }

    result
}

/// Append a bullet point segment with proper styling
fn append_bullet_point(
    result: &NSMutableAttributedString,
    text: &str,
    text_color: &NSColor,
    font: &NSFont,
) {
    let bullet_text = format!("  •  {}", text);
    let ns_str = NSString::from_str(&bullet_text);
    let segment_attr: Retained<NSMutableAttributedString> =
        unsafe { msg_send_id![NSMutableAttributedString::alloc(), initWithString: &*ns_str] };
    let len: usize = unsafe { msg_send![&segment_attr, length] };
    let range = NSRange::new(0, len);
    let color_attr = NSString::from_str("NSColor");
    let font_attr = NSString::from_str("NSFont");
    unsafe {
        let _: () =
            msg_send![&segment_attr, addAttribute: &*color_attr, value: text_color, range: range];
        let _: () = msg_send![&segment_attr, addAttribute: &*font_attr, value: font, range: range];
        let _: () = msg_send![result, appendAttributedString: &*segment_attr];
    }
}

/// Append styled text to the attributed string
fn append_styled_text(
    result: &NSMutableAttributedString,
    text_content: &str,
    font: &NSFont,
    text_color: &NSColor,
    color_attr: &NSString,
    font_attr: &NSString,
) {
    let ns_str = NSString::from_str(text_content);
    let segment_attr: Retained<NSMutableAttributedString> =
        unsafe { msg_send_id![NSMutableAttributedString::alloc(), initWithString: &*ns_str] };

    let len: usize = unsafe { msg_send![&segment_attr, length] };
    if len > 0 {
        let range = NSRange::new(0, len);
        unsafe {
            let _: () =
                msg_send![&segment_attr, addAttribute: color_attr, value: text_color, range: range];
            let _: () =
                msg_send![&segment_attr, addAttribute: font_attr, value: font, range: range];
            let _: () = msg_send![result, appendAttributedString: &*segment_attr];
        }
    }
}
