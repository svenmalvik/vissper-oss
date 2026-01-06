//! Menu building logic
//!
//! Functions for constructing the menu structure and its items.

use objc2::rc::Retained;
use objc2::sel;
use objc2_app_kit::{NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSString};
use std::sync::atomic::Ordering;
use tracing::info;

use super::delegate::VissperMenuDelegate;
use super::items::{create_menu_item, create_menu_item_with_key};
use super::APP_STATE;
use crate::preferences;

/// Build all menu items and add them to the menu
#[allow(clippy::type_complexity)]
pub(super) fn build_menu_items(
    mtm: MainThreadMarker,
    menu: &NSMenu,
    delegate: &VissperMenuDelegate,
) -> (
    Retained<NSMenuItem>,
    Retained<NSMenu>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>, // screenshots_item
    Retained<NSMenu>,     // screenshots_submenu
    Retained<NSMenuItem>, // screenshot_fullscreen_item
    Retained<NSMenuItem>, // screenshot_region_item
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>, // update_available_item
) {
    // Recording item with keyboard shortcut
    let recording_item = create_menu_item_with_key(
        mtm,
        "Start Recording",
        sel!(handleStartRecording:),
        delegate,
        " ",
        262144,
    );
    menu.addItem(&recording_item);

    // Stop recording submenu
    let stop_submenu = NSMenu::new(mtm);
    unsafe { stop_submenu.setAutoenablesItems(false) };

    let stop_no_polish_item = create_menu_item_with_key(
        mtm,
        "No polishing",
        sel!(handleStopNoPolish:),
        delegate,
        " ",
        262144,
    );
    stop_submenu.addItem(&stop_no_polish_item);

    let stop_basic_polish_item = create_menu_item_with_key(
        mtm,
        "Basic polishing",
        sel!(handleStopBasicPolish:),
        delegate,
        "1",
        393216,
    );
    stop_submenu.addItem(&stop_basic_polish_item);

    let stop_meeting_notes_item = create_menu_item_with_key(
        mtm,
        "Meeting notes",
        sel!(handleStopMeetingNotes:),
        delegate,
        "2",
        393216,
    );
    stop_submenu.addItem(&stop_meeting_notes_item);

    // Show Window item
    let show_window_item =
        create_menu_item(mtm, "Show Transcription", sel!(handleShowWindow:), delegate);
    menu.addItem(&show_window_item);

    // Screenshots submenu
    let screenshots_submenu = NSMenu::new(mtm);
    unsafe { screenshots_submenu.setAutoenablesItems(false) };

    // Capture Entire Screen (Ctrl+Shift+0)
    let screenshot_fullscreen_item = create_menu_item_with_key(
        mtm,
        "Capture Entire Screen",
        sel!(handleScreenshot:),
        delegate,
        "0",
        393216,
    );
    screenshots_submenu.addItem(&screenshot_fullscreen_item);

    // Capture Selected Area (Ctrl+Shift+9)
    let screenshot_region_item = create_menu_item_with_key(
        mtm,
        "Capture Selected Area",
        sel!(handleRegionScreenshot:),
        delegate,
        "9",
        393216,
    );
    screenshots_submenu.addItem(&screenshot_region_item);

    // Create Screenshots parent menu item (no action, just shows submenu)
    let screenshots_item = {
        let title_str = NSString::from_str("Screenshots");
        let key = NSString::from_str("");
        unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &title_str, None, &key)
        }
    };
    screenshots_item.setSubmenu(Some(&screenshots_submenu));
    menu.addItem(&screenshots_item);

    menu.addItem(&NSMenuItem::separatorItem(mtm));

    // Settings item
    let settings_item = create_menu_item(mtm, "Settings", sel!(handleSettings:), delegate);
    menu.addItem(&settings_item);

    menu.addItem(&NSMenuItem::separatorItem(mtm));

    // Languages submenu
    let (
        languages_item,
        lang_english_item,
        lang_norwegian_item,
        lang_danish_item,
        lang_finnish_item,
        lang_german_item,
    ) = build_languages_submenu(mtm, menu, delegate);

    menu.addItem(&NSMenuItem::separatorItem(mtm));

    // About item
    let about_item = create_menu_item(mtm, "About Vissper", sel!(handleAbout:), delegate);
    menu.addItem(&about_item);

    // Update Available item (initially hidden)
    let update_available_item = create_menu_item(
        mtm,
        "", // Empty title initially
        sel!(handleUpdateAvailable:),
        delegate,
    );
    unsafe { update_available_item.setHidden(true) }; // Hidden by default
    menu.addItem(&update_available_item);

    // Quit item
    let quit_item = create_menu_item(mtm, "Quit Vissper", sel!(handleQuit:), delegate);
    menu.addItem(&quit_item);

    (
        recording_item,
        stop_submenu,
        stop_no_polish_item,
        stop_basic_polish_item,
        stop_meeting_notes_item,
        show_window_item,
        screenshots_item,
        screenshots_submenu,
        screenshot_fullscreen_item,
        screenshot_region_item,
        settings_item,
        languages_item,
        lang_english_item,
        lang_norwegian_item,
        lang_danish_item,
        lang_finnish_item,
        lang_german_item,
        update_available_item,
    )
}

/// Build the languages submenu
#[allow(clippy::type_complexity)]
pub(super) fn build_languages_submenu(
    mtm: MainThreadMarker,
    menu: &NSMenu,
    delegate: &VissperMenuDelegate,
) -> (
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
    Retained<NSMenuItem>,
) {
    let languages_menu = NSMenu::new(mtm);
    unsafe { languages_menu.setAutoenablesItems(false) };

    let lang_english_item =
        create_menu_item(mtm, "English", sel!(handleLanguageEnglish:), delegate);
    languages_menu.addItem(&lang_english_item);

    let lang_norwegian_item =
        create_menu_item(mtm, "Norwegian", sel!(handleLanguageNorwegian:), delegate);
    languages_menu.addItem(&lang_norwegian_item);

    let lang_danish_item = create_menu_item(mtm, "Danish", sel!(handleLanguageDanish:), delegate);
    languages_menu.addItem(&lang_danish_item);

    let lang_finnish_item =
        create_menu_item(mtm, "Finnish", sel!(handleLanguageFinnish:), delegate);
    languages_menu.addItem(&lang_finnish_item);

    let lang_german_item = create_menu_item(mtm, "German", sel!(handleLanguageGerman:), delegate);
    languages_menu.addItem(&lang_german_item);

    // Create Languages menu item and attach submenu
    let languages_item = {
        let title_str = NSString::from_str("Languages");
        let key = NSString::from_str("");
        unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &title_str, None, &key)
        }
    };
    languages_item.setSubmenu(Some(&languages_menu));
    menu.addItem(&languages_item);

    // Set initial checkmarks
    update_language_checkmarks_for_items(
        &lang_english_item,
        &lang_norwegian_item,
        &lang_danish_item,
        &lang_finnish_item,
        &lang_german_item,
    );

    (
        languages_item,
        lang_english_item,
        lang_norwegian_item,
        lang_danish_item,
        lang_finnish_item,
        lang_german_item,
    )
}

/// Apply initial UI state to menu items
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_initial_state(
    recording_item: &NSMenuItem,
    settings_item: &NSMenuItem,
    show_window_item: &NSMenuItem,
    screenshots_item: &NSMenuItem,
    screenshot_fullscreen_item: &NSMenuItem,
    screenshot_region_item: &NSMenuItem,
    languages_item: &NSMenuItem,
) {
    if let Some(state) = APP_STATE.get() {
        let has_azure_credentials = state.has_azure_credentials.load(Ordering::SeqCst);

        info!(
            "Initial state: has_azure_credentials={}",
            has_azure_credentials
        );

        if !has_azure_credentials {
            info!("Disabling recording (no Azure credentials)");
            unsafe {
                recording_item.setEnabled(false);
            }
        }

        // These items are always enabled in OSS version
        unsafe {
            settings_item.setEnabled(true);
            show_window_item.setEnabled(true);
            screenshots_item.setEnabled(true);
            screenshot_fullscreen_item.setEnabled(true);
            screenshot_region_item.setEnabled(true);
            languages_item.setEnabled(true);
        }
    }
}

/// Update checkmarks for the given language menu items
pub(super) fn update_language_checkmarks_for_items(
    english: &NSMenuItem,
    norwegian: &NSMenuItem,
    danish: &NSMenuItem,
    finnish: &NSMenuItem,
    german: &NSMenuItem,
) {
    let current_lang = preferences::get_language_code();

    unsafe {
        english.setState(if current_lang == "en" { 1 } else { 0 });
        norwegian.setState(if current_lang == "no" { 1 } else { 0 });
        danish.setState(if current_lang == "da" { 1 } else { 0 });
        finnish.setState(if current_lang == "fi" { 1 } else { 0 });
        german.setState(if current_lang == "de" { 1 } else { 0 });
    }
}
