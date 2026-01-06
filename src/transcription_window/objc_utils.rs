//! Objective-C runtime utilities for safer class lookups
//!
//! Provides graceful handling of Objective-C class lookups that logs warnings
//! instead of panicking when classes are not found.

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2_app_kit::NSView;
use tracing::warn;

/// Safely get an Objective-C class, logging a warning if it doesn't exist.
///
/// This is used for classes that are guaranteed to exist on supported macOS versions
/// but should not panic if they're missing (graceful degradation).
///
/// # Arguments
/// * `class_name` - The name of the Objective-C class to look up.
///
/// # Returns
/// * `Some(&AnyClass)` if the class was found.
/// * `None` if the class was not found (warning logged).
pub(super) fn get_class_or_warn(class_name: &str) -> Option<&'static AnyClass> {
    match AnyClass::get(class_name) {
        Some(class) => Some(class),
        None => {
            warn!(
                "Objective-C class '{}' not found - feature may be unavailable",
                class_name
            );
            None
        }
    }
}

/// Safely retain a pointer as an NSView, logging if the retain fails.
///
/// # Safety
/// The caller must ensure that `ptr` points to a valid Objective-C object
/// that is an NSView or subclass of NSView.
///
/// # Arguments
/// * `ptr` - A raw pointer to an Objective-C object to retain as NSView.
///
/// # Returns
/// * `Some(Retained<NSView>)` if the retain succeeded.
/// * `None` if the pointer was null or the retain failed (warning logged).
pub(super) unsafe fn retain_as_view(ptr: *mut AnyObject) -> Option<Retained<NSView>> {
    if ptr.is_null() {
        warn!("Cannot retain null pointer as NSView");
        return None;
    }
    match Retained::retain(ptr as *mut NSView) {
        Some(view) => Some(view),
        None => {
            warn!("Failed to retain pointer as NSView");
            None
        }
    }
}
