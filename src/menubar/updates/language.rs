//! Language selection functions
//!
//! Functions for setting and updating the transcription language.

use crate::menubar::builder::update_language_checkmarks_for_items;
use crate::menubar::MENU_BAR;
use crate::preferences;

/// Set the transcription language and update the menu checkmarks
pub fn set_language(code: &str) {
    if let Err(e) = preferences::set_language_code(code) {
        tracing::error!("Failed to save language preference: {}", e);
    }
    update_language_checkmarks();
}

/// Update language menu checkmarks based on current preference
fn update_language_checkmarks() {
    let Some(menu_bar) = MENU_BAR.get() else {
        return;
    };
    let Ok(inner) = menu_bar.lock() else {
        return;
    };

    update_language_checkmarks_for_items(
        &inner.lang_english_item,
        &inner.lang_norwegian_item,
        &inner.lang_danish_item,
        &inner.lang_finnish_item,
        &inner.lang_german_item,
    );
}
