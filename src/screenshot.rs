//! Screenshot capture module
//!
//! Provides functionality to capture screenshots during Live Meeting Recording.
//! Screenshots are saved to a configurable directory (default: ~/Documents/Vissper/screenshots).
//!
//! Uses macOS `screencapture` command which properly handles Spaces (virtual desktops).

use crate::preferences;
use arboard::Clipboard;
use chrono::Local;
use image::ImageReader;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{error, info};

/// Capture a screenshot and save it to the screenshots folder
///
/// Uses macOS `screencapture` command which correctly captures the currently
/// visible Space/desktop, unlike CGDisplay which may capture the wrong desktop.
///
/// Returns the filename of the screenshot (e.g., "screenshot-2025-12-11-14-30-45.png")
/// for embedding in the transcript as a markdown image reference.
///
/// # Returns
/// - `Ok(filename)` - The filename of the saved screenshot
/// - `Err(message)` - Error message if capture or save failed
pub(crate) fn capture_screenshot() -> Result<String, ScreenshotError> {
    // Get the screenshots directory
    let screenshots_dir = ensure_screenshots_dir()?;

    // Generate filename with timestamp
    let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S");
    let filename = format!("screenshot-{}.png", timestamp);
    let filepath = screenshots_dir.join(&filename);
    let filepath_str = filepath.to_string_lossy().to_string();

    // Use macOS screencapture command
    // -x: no sound
    // -t png: format
    // -C: capture cursor (optional, remove if not wanted)
    let output = Command::new("screencapture")
        .args(["-x", "-t", "png", &filepath_str])
        .output()
        .map_err(|e| {
            ScreenshotError::CaptureError(format!("Failed to run screencapture: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("screencapture failed: {}", stderr);
        return Err(ScreenshotError::CaptureError(format!(
            "screencapture exited with status {}: {}",
            output.status, stderr
        )));
    }

    // Verify file was created
    if !filepath.exists() {
        return Err(ScreenshotError::SaveError(
            "Screenshot file was not created".into(),
        ));
    }

    info!("Screenshot saved to: {:?}", filepath);

    // Copy to clipboard
    copy_to_clipboard(&filepath);

    // Return filename for markdown embedding
    Ok(filename)
}

/// Capture a screenshot of a specific region
///
/// Uses macOS `screencapture -R x,y,w,h` command to capture only the specified
/// rectangular region. Coordinates use top-left origin (screencapture convention).
///
/// # Arguments
/// * `x` - X coordinate of region origin (from left edge)
/// * `y` - Y coordinate of region origin (from top edge)
/// * `width` - Width of region in points
/// * `height` - Height of region in points
///
/// # Returns
/// - `Ok(filename)` - The filename of the saved screenshot
/// - `Err(ScreenshotError)` - Error if capture or save failed
pub(crate) fn capture_region_screenshot(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<String, ScreenshotError> {
    // Get the screenshots directory
    let screenshots_dir = ensure_screenshots_dir()?;

    // Generate filename with timestamp
    let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S");
    let filename = format!("screenshot-{}.png", timestamp);
    let filepath = screenshots_dir.join(&filename);
    let filepath_str = filepath.to_string_lossy().to_string();

    // Format region as x,y,width,height (integers)
    let region = format!(
        "{},{},{},{}",
        x as i32, y as i32, width as i32, height as i32
    );

    // Use macOS screencapture command with region flag
    // -x: no sound
    // -t png: format
    // -R x,y,w,h: capture specific region
    let output = Command::new("screencapture")
        .args(["-x", "-t", "png", "-R", &region, &filepath_str])
        .output()
        .map_err(|e| {
            ScreenshotError::CaptureError(format!("Failed to run screencapture: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("screencapture region failed: {}", stderr);
        return Err(ScreenshotError::CaptureError(format!(
            "screencapture exited with status {}: {}",
            output.status, stderr
        )));
    }

    // Verify file was created
    if !filepath.exists() {
        return Err(ScreenshotError::SaveError(
            "Region screenshot file was not created".into(),
        ));
    }

    info!("Region screenshot saved to: {:?}", filepath);

    // Copy to clipboard
    copy_to_clipboard(&filepath);

    // Return filename for markdown embedding
    Ok(filename)
}

/// Ensure the screenshots directory exists
///
/// Uses custom screenshot location from preferences if set,
/// otherwise falls back to default location (~/Documents/Vissper/screenshots).
fn ensure_screenshots_dir() -> Result<PathBuf, ScreenshotError> {
    let screenshots_dir = preferences::get_screenshot_location()
        .or_else(preferences::default_screenshot_location)
        .ok_or(ScreenshotError::NoScreenshotsDir)?;

    if !screenshots_dir.exists() {
        fs::create_dir_all(&screenshots_dir)?;
        info!("Created screenshots directory: {:?}", screenshots_dir);
    }

    Ok(screenshots_dir)
}

/// Copy a screenshot image file to the system clipboard
///
/// Uses arboard to copy the PNG file to the clipboard so users can
/// paste the screenshot directly into other applications.
fn copy_to_clipboard(filepath: &Path) {
    // Read and decode the image
    let img = match ImageReader::open(filepath) {
        Ok(reader) => match reader.decode() {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                error!("Failed to decode screenshot for clipboard: {}", e);
                return;
            }
        },
        Err(e) => {
            error!("Failed to open screenshot for clipboard: {}", e);
            return;
        }
    };

    let (width, height) = img.dimensions();
    let rgba_data = img.into_raw();

    // Create arboard ImageData
    let image_data = arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: rgba_data.into(),
    };

    // Copy to clipboard
    match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_image(image_data) {
            Ok(_) => {
                info!("Screenshot copied to clipboard");
            }
            Err(e) => {
                error!("Failed to copy screenshot to clipboard: {}", e);
            }
        },
        Err(e) => {
            error!("Failed to initialize clipboard: {}", e);
        }
    }
}

/// Screenshot errors
#[derive(Debug, thiserror::Error)]
pub(crate) enum ScreenshotError {
    #[error("Could not determine screenshots directory")]
    NoScreenshotsDir,

    #[error("Screenshot capture failed: {0}")]
    CaptureError(String),

    #[error("Failed to save screenshot: {0}")]
    SaveError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
