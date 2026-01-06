//! macOS Menu Bar (Status Bar) implementation using objc2
//!
//! This module provides a native macOS menu bar item with a dropdown menu
//! for the Vissper application.

mod builder;
mod delegate;
mod icons;
mod items;
mod state;
mod updates;

pub use state::{AppState, MenuCallbacks};

use builder::{apply_initial_state, build_menu_items};
use delegate::VissperMenuDelegate;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
};
use objc2_foundation::MainThreadMarker;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};

/// Global state for menu bar (needed for Objective-C callbacks)
pub(super) static MENU_BAR: OnceCell<Mutex<MenuBarInner>> = OnceCell::new();
pub(super) static APP_STATE: OnceCell<Arc<AppState>> = OnceCell::new();
pub(super) static CALLBACKS: OnceCell<MenuCallbacks> = OnceCell::new();

/// Inner menu bar state holding retained references
pub(super) struct MenuBarInner {
    pub(super) status_item: Retained<NSStatusItem>,
    #[allow(dead_code)]
    menu: Retained<NSMenu>,
    #[allow(dead_code)]
    delegate: Retained<VissperMenuDelegate>,
    pub(super) recording_item: Retained<NSMenuItem>,
    pub(super) stop_submenu: Retained<NSMenu>,
    #[allow(dead_code)]
    stop_no_polish_item: Retained<NSMenuItem>,
    #[allow(dead_code)]
    stop_basic_polish_item: Retained<NSMenuItem>,
    #[allow(dead_code)]
    stop_meeting_notes_item: Retained<NSMenuItem>,
    pub(super) show_window_item: Retained<NSMenuItem>,
    pub(super) screenshots_item: Retained<NSMenuItem>,
    #[allow(dead_code)]
    pub(super) screenshots_submenu: Retained<NSMenu>,
    pub(super) screenshot_fullscreen_item: Retained<NSMenuItem>,
    pub(super) screenshot_region_item: Retained<NSMenuItem>,
    pub(super) settings_item: Retained<NSMenuItem>,
    pub(super) languages_item: Retained<NSMenuItem>,
    pub(super) lang_english_item: Retained<NSMenuItem>,
    pub(super) lang_norwegian_item: Retained<NSMenuItem>,
    pub(super) lang_danish_item: Retained<NSMenuItem>,
    pub(super) lang_finnish_item: Retained<NSMenuItem>,
    pub(super) lang_german_item: Retained<NSMenuItem>,
    pub(super) update_available_item: Retained<NSMenuItem>,
}

unsafe impl Send for MenuBarInner {}

/// Menu bar manager
pub struct MenuBar;

impl MenuBar {
    /// Initialize the menu bar with the given state and callbacks
    pub fn init(state: Arc<AppState>, callbacks: MenuCallbacks) {
        APP_STATE.set(state).ok();
        CALLBACKS.set(callbacks).ok();

        let mtm = MainThreadMarker::new().expect(
            "MenuBar::init() must be called on the main thread - ensure this is called from main()",
        );

        // Get the shared application and set as accessory (menu bar only, no dock icon)
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

        // Create a main menu bar with Edit menu for keyboard shortcuts (Cmd+V, Cmd+C, etc.)
        Self::setup_edit_menu(mtm, &app);

        // Create the delegate
        let delegate = VissperMenuDelegate::new(mtm);

        // Create status item
        let status_bar = unsafe { NSStatusBar::systemStatusBar() };
        let status_item = unsafe { status_bar.statusItemWithLength(-2.0) };

        // Set initial icon (idle state)
        icons::set_icon(&status_item, false, false, mtm);

        // Create menu and disable auto-enabling so we control enabled state
        let menu = NSMenu::new(mtm);
        unsafe { menu.setAutoenablesItems(false) };

        // Build menu items
        let (
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
        ) = build_menu_items(mtm, &menu, &delegate);

        // Attach menu to status item
        unsafe { status_item.setMenu(Some(&menu)) };

        // Apply initial UI state
        apply_initial_state(
            &recording_item,
            &settings_item,
            &show_window_item,
            &screenshots_item,
            &screenshot_fullscreen_item,
            &screenshot_region_item,
            &languages_item,
        );

        // Store in global state
        let inner = MenuBarInner {
            status_item,
            menu,
            delegate,
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
        };

        MENU_BAR.set(Mutex::new(inner)).ok();
    }

    /// Update the menu bar UI based on current state
    #[allow(dead_code)]
    pub fn update_ui() {
        updates::update_ui();
    }

    /// Set Azure credentials state (thread-safe)
    ///
    /// When Azure credentials are available, recording is enabled.
    pub fn set_azure_credentials(available: bool) {
        updates::set_azure_credentials(available);
    }

    /// Set recording state (thread-safe)
    pub fn set_recording(recording: bool) {
        updates::set_recording(recording);
    }

    /// Set processing state (thread-safe)
    pub fn set_processing(processing: bool) {
        updates::set_processing(processing);
    }

    /// Set the transcription language and update the menu checkmarks
    pub fn set_language(code: &str) {
        updates::set_language(code);
    }

    /// Show update available menu item (thread-safe)
    pub fn show_update_available(version: &str) {
        updates::show_update_available(version);
    }

    /// Hide update available menu item (thread-safe)
    pub fn hide_update_available() {
        updates::hide_update_available();
    }

    /// Run the application event loop
    pub fn run() {
        let mtm = MainThreadMarker::new().expect(
            "MenuBar::run() must be called on the main thread - ensure this is called from main()",
        );
        let app = NSApplication::sharedApplication(mtm);
        unsafe { app.run() };
    }

    /// Stop the application
    pub fn stop() {
        if let Some(mtm) = MainThreadMarker::new() {
            let app = NSApplication::sharedApplication(mtm);
            app.stop(None);
        }
    }

    /// Setup the main menu bar with Edit menu for keyboard shortcuts.
    ///
    /// Even accessory apps need a main menu bar for standard keyboard shortcuts
    /// (Cmd+V, Cmd+C, Cmd+X, Cmd+A, Cmd+Z) to work in text fields.
    fn setup_edit_menu(mtm: MainThreadMarker, app: &NSApplication) {
        use objc2::{msg_send, sel};
        use objc2_foundation::NSString;

        // Create main menu bar
        let main_menu = NSMenu::new(mtm);

        // Create Edit menu
        let edit_menu = NSMenu::new(mtm);
        unsafe { edit_menu.setTitle(&NSString::from_str("Edit")) };

        // Add standard editing items with keyboard shortcuts
        // Undo - Cmd+Z
        let undo_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &NSString::from_str("Undo"),
                Some(sel!(undo:)),
                &NSString::from_str("z"),
            )
        };
        edit_menu.addItem(&undo_item);

        // Redo - Cmd+Shift+Z
        let redo_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &NSString::from_str("Redo"),
                Some(sel!(redo:)),
                &NSString::from_str("Z"),
            )
        };
        edit_menu.addItem(&redo_item);

        // Separator
        edit_menu.addItem(&NSMenuItem::separatorItem(mtm));

        // Cut - Cmd+X
        let cut_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &NSString::from_str("Cut"),
                Some(sel!(cut:)),
                &NSString::from_str("x"),
            )
        };
        edit_menu.addItem(&cut_item);

        // Copy - Cmd+C
        let copy_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &NSString::from_str("Copy"),
                Some(sel!(copy:)),
                &NSString::from_str("c"),
            )
        };
        edit_menu.addItem(&copy_item);

        // Paste - Cmd+V
        let paste_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &NSString::from_str("Paste"),
                Some(sel!(paste:)),
                &NSString::from_str("v"),
            )
        };
        edit_menu.addItem(&paste_item);

        // Select All - Cmd+A
        let select_all_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &NSString::from_str("Select All"),
                Some(sel!(selectAll:)),
                &NSString::from_str("a"),
            )
        };
        edit_menu.addItem(&select_all_item);

        // Create Edit menu item for main menu bar
        let edit_menu_item = NSMenuItem::new(mtm);
        edit_menu_item.setSubmenu(Some(&edit_menu));

        // Add Edit menu to main menu bar
        main_menu.addItem(&edit_menu_item);

        // Set as application's main menu
        unsafe {
            let _: () = msg_send![app, setMainMenu: &*main_menu];
        }
    }
}
