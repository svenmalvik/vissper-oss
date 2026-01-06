//! Menu item creation helpers
//!
//! Provides utility functions for creating NSMenuItem instances.

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2_app_kit::NSMenuItem;
use objc2_foundation::{MainThreadMarker, NSString};

use super::delegate::VissperMenuDelegate;

/// Create a menu item with the given title and action
pub(super) fn create_menu_item(
    mtm: MainThreadMarker,
    title: &str,
    action: Sel,
    target: &VissperMenuDelegate,
) -> Retained<NSMenuItem> {
    create_menu_item_with_key(mtm, title, action, target, "", 0)
}

/// Create a menu item with the given title, action, and key equivalent
pub(super) fn create_menu_item_with_key(
    mtm: MainThreadMarker,
    title: &str,
    action: Sel,
    target: &VissperMenuDelegate,
    key_equivalent: &str,
    modifier_mask: u64,
) -> Retained<NSMenuItem> {
    let title_str = NSString::from_str(title);
    let key = NSString::from_str(key_equivalent);

    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &title_str, Some(action), &key)
    };

    if modifier_mask != 0 {
        unsafe {
            let _: () = msg_send![&item, setKeyEquivalentModifierMask: modifier_mask];
        }
    }

    unsafe {
        let _: () = msg_send![&item, setTarget: target];
    }

    item
}
