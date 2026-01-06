//! App update availability functions
//!
//! Functions for showing/hiding the update available menu item.

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::ClassType;
use objc2_app_kit::{NSColor, NSFont};
use objc2_foundation::{MainThreadMarker, NSMutableAttributedString, NSRange, NSString};

use crate::menubar::MENU_BAR;

/// Show update available menu item (thread-safe)
pub fn show_update_available(version: &str) {
    let title = format!("Update Available: v{}", version);

    if MainThreadMarker::new().is_some() {
        update_update_item(&title, false);
    } else {
        let title_owned = title.clone();
        dispatch::Queue::main().exec_async(move || {
            update_update_item(&title_owned, false);
        });
    }
}

/// Hide update available menu item (thread-safe)
pub fn hide_update_available() {
    if MainThreadMarker::new().is_some() {
        update_update_item("", true);
    } else {
        dispatch::Queue::main().exec_async(|| {
            update_update_item("", true);
        });
    }
}

/// Update the update item (must be called on main thread)
fn update_update_item(title: &str, hidden: bool) {
    let Some(menu_bar) = MENU_BAR.get() else {
        return;
    };
    let Ok(inner) = menu_bar.lock() else {
        return;
    };

    unsafe {
        if hidden {
            // Just hide the item, no need to set attributed title
            let title_str = NSString::from_str(title);
            inner.update_available_item.setTitle(&title_str);
        } else {
            // Create attributed string with orange color to highlight the update
            let ns_str = NSString::from_str(title);
            let attr_string: Retained<NSMutableAttributedString> =
                objc2::msg_send_id![NSMutableAttributedString::alloc(), initWithString: &*ns_str];

            let len: usize = msg_send![&attr_string, length];
            let range = NSRange::new(0, len);

            // Use system orange color for visibility
            let orange_color = NSColor::systemOrangeColor();
            let font = NSFont::menuFontOfSize(0.0); // 0 = default menu font size

            let color_attr = NSString::from_str("NSColor");
            let font_attr = NSString::from_str("NSFont");

            let _: () = msg_send![&attr_string, addAttribute: &*color_attr, value: &*orange_color, range: range];
            let _: () =
                msg_send![&attr_string, addAttribute: &*font_attr, value: &*font, range: range];

            inner
                .update_available_item
                .setAttributedTitle(Some(&attr_string));
        }
        inner.update_available_item.setHidden(hidden);
    }
}
