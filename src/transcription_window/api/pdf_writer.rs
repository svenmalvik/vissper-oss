//! PDF generation from markdown content.
//!
//! Uses genpdf to render markdown-formatted transcripts to PDF files
//! with proper styling (headers, bold, bullet points).

use std::path::Path;

use anyhow::{Context, Result};
use genpdf::elements::{Break, Paragraph};
use genpdf::fonts::{FontData, FontFamily};
use genpdf::style::{Style, StyledString};
use genpdf::{Document, Margins, SimplePageDecorator};
use tracing::info;

use crate::transcription_window::markdown::{parse_markdown, MarkdownSegment};

/// Font sizes for PDF output (in points).
const NORMAL_SIZE: u8 = 11;
const BOLD_SIZE: u8 = 11;
const H1_SIZE: u8 = 18;
const H2_SIZE: u8 = 14;
const H3_SIZE: u8 = 12;

/// Page margins in mm.
const MARGIN_MM: f64 = 20.0;

/// Write markdown-formatted content to a PDF file.
///
/// Parses the markdown content and renders it with styled formatting:
/// - Headers (H1/H2/H3) with appropriate sizes
/// - Bold text preserved
/// - Bullet points with indentation
///
/// # Errors
///
/// Returns an error if:
/// - No suitable font can be loaded from the system
/// - The PDF file cannot be written to the specified path
pub(crate) fn write_pdf(path: &Path, content: &str) -> Result<()> {
    info!(
        path = %path.display(),
        content_length = content.len(),
        "Generating PDF transcript"
    );

    // Load font family from system fonts
    let font_family =
        load_font_family().with_context(|| "Failed to load system font for PDF generation")?;

    // Create document with default font
    let mut doc = Document::new(font_family);
    doc.set_title("Vissper Transcript");

    // Set page margins
    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(MARGIN_MM, MARGIN_MM, MARGIN_MM, MARGIN_MM));
    doc.set_page_decorator(decorator);

    // Parse markdown and add elements
    let segments = parse_markdown(content);

    for segment in segments {
        match segment {
            MarkdownSegment::Header1(text) => {
                let style = Style::new().bold().with_font_size(H1_SIZE);
                doc.push(Paragraph::new(StyledString::new(text, style)));
            }
            MarkdownSegment::Header2(text) => {
                let style = Style::new().bold().with_font_size(H2_SIZE);
                doc.push(Paragraph::new(StyledString::new(text, style)));
            }
            MarkdownSegment::Header3(text) => {
                let style = Style::new().bold().with_font_size(H3_SIZE);
                doc.push(Paragraph::new(StyledString::new(text, style)));
            }
            MarkdownSegment::BulletPoint(text) => {
                let bullet_text = format!("  \u{2022}  {}", text);
                let style = Style::new().with_font_size(NORMAL_SIZE);
                doc.push(Paragraph::new(StyledString::new(bullet_text, style)));
            }
            MarkdownSegment::Bold(text) => {
                let style = Style::new().bold().with_font_size(BOLD_SIZE);
                doc.push(Paragraph::new(StyledString::new(text, style)));
            }
            MarkdownSegment::Normal(text) => {
                if text == "\n" {
                    doc.push(Break::new(0.5));
                } else {
                    let style = Style::new().with_font_size(NORMAL_SIZE);
                    doc.push(Paragraph::new(StyledString::new(text, style)));
                }
            }
        }
    }

    // Render to file
    doc.render_to_file(path)
        .with_context(|| format!("Failed to render PDF to {}", path.display()))?;

    info!(path = %path.display(), "PDF transcript saved successfully");
    Ok(())
}

/// Load a font family for PDF generation.
///
/// Loads Arial fonts from macOS system font locations.
fn load_font_family() -> Result<FontFamily<FontData>> {
    let font_dir = Path::new("/System/Library/Fonts/Supplemental");

    // Load regular font
    let regular_path = font_dir.join("Arial.ttf");
    let regular = FontData::new(
        std::fs::read(&regular_path)
            .with_context(|| format!("Failed to read font: {}", regular_path.display()))?,
        None,
    )
    .with_context(|| "Failed to parse Arial regular font")?;

    // Load bold font
    let bold_path = font_dir.join("Arial Bold.ttf");
    let bold = FontData::new(
        std::fs::read(&bold_path)
            .with_context(|| format!("Failed to read font: {}", bold_path.display()))?,
        None,
    )
    .with_context(|| "Failed to parse Arial bold font")?;

    // Load italic font
    let italic_path = font_dir.join("Arial Italic.ttf");
    let italic = FontData::new(
        std::fs::read(&italic_path)
            .with_context(|| format!("Failed to read font: {}", italic_path.display()))?,
        None,
    )
    .with_context(|| "Failed to parse Arial italic font")?;

    // Load bold italic font
    let bold_italic_path = font_dir.join("Arial Bold Italic.ttf");
    let bold_italic = FontData::new(
        std::fs::read(&bold_italic_path)
            .with_context(|| format!("Failed to read font: {}", bold_italic_path.display()))?,
        None,
    )
    .with_context(|| "Failed to parse Arial bold italic font")?;

    Ok(FontFamily {
        regular,
        bold,
        italic,
        bold_italic,
    })
}
