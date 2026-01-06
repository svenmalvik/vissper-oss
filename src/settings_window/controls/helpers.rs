//! Common UI helper functions for creating settings window controls.

use objc2::rc::Retained;
use objc2::{msg_send, msg_send_id};
use objc2_app_kit::{
    NSBox, NSBoxType, NSButton, NSFont, NSSegmentedControl, NSSlider, NSTabView, NSTabViewItem,
    NSTextField,
};
use objc2_foundation::{CGFloat, MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

use crate::settings_window::constants::NS_BEZEL_STYLE_ROUNDED;
use crate::settings_window::delegate::SettingsActionDelegate;

/// Create a section label (bold, non-editable text field).
pub(crate) fn create_section_label(
    mtm: MainThreadMarker,
    frame: NSRect,
    text: &str,
) -> Retained<NSTextField> {
    // SAFETY: NSTextField allocation and initialization is safe on main thread with valid frame
    let label: Retained<NSTextField> =
        unsafe { msg_send_id![mtm.alloc::<NSTextField>(), initWithFrame: frame] };

    // SAFETY: Standard NSTextField configuration calls
    unsafe {
        label.setEditable(false);
        label.setSelectable(false);
        label.setBordered(false);
        label.setDrawsBackground(false);
        label.setStringValue(&NSString::from_str(text));

        let font = NSFont::boldSystemFontOfSize(13.0);
        label.setFont(Some(&font));
    }

    label
}

/// Create a path display label (smaller font, selectable).
pub(crate) fn create_path_label(
    mtm: MainThreadMarker,
    frame: NSRect,
    path: &str,
) -> Retained<NSTextField> {
    // SAFETY: NSTextField allocation and initialization is safe on main thread with valid frame
    let label: Retained<NSTextField> =
        unsafe { msg_send_id![mtm.alloc::<NSTextField>(), initWithFrame: frame] };

    // SAFETY: Standard NSTextField configuration calls
    unsafe {
        label.setEditable(false);
        label.setSelectable(true);
        label.setBordered(false);
        label.setDrawsBackground(false);
        label.setStringValue(&NSString::from_str(path));

        let font = NSFont::systemFontOfSize(11.0);
        label.setFont(Some(&font));
    }

    label
}

/// Create a standard button with the given title and action.
pub(crate) fn create_button(
    mtm: MainThreadMarker,
    frame: NSRect,
    title: &str,
    delegate: &SettingsActionDelegate,
    action: objc2::runtime::Sel,
) -> Retained<NSButton> {
    // SAFETY: NSButton allocation and initialization is safe on main thread with valid frame
    let button: Retained<NSButton> =
        unsafe { msg_send_id![mtm.alloc::<NSButton>(), initWithFrame: frame] };

    // SAFETY: Standard NSButton configuration with valid delegate target
    unsafe {
        let ns_title = NSString::from_str(title);
        let _: () = msg_send![&button, setTitle: &*ns_title];
        let _: () = msg_send![&button, setBezelStyle: NS_BEZEL_STYLE_ROUNDED];
        let _: () = msg_send![&button, setTarget: delegate];
        let _: () = msg_send![&button, setAction: action];
    }

    button
}

/// Create a button with smaller font (for location controls).
pub(crate) fn create_small_button(
    mtm: MainThreadMarker,
    frame: NSRect,
    title: &str,
    delegate: &SettingsActionDelegate,
    action: objc2::runtime::Sel,
) -> Retained<NSButton> {
    let button = create_button(mtm, frame, title, delegate, action);

    // SAFETY: Setting font on a valid NSButton
    unsafe {
        let font = NSFont::systemFontOfSize(12.0);
        button.setFont(Some(&font));
    }

    button
}

/// Create a horizontal slider control.
pub(crate) fn create_slider(
    mtm: MainThreadMarker,
    frame: NSRect,
    min_value: f64,
    max_value: f64,
    current_value: f64,
    delegate: &SettingsActionDelegate,
    action: objc2::runtime::Sel,
) -> Retained<NSSlider> {
    // SAFETY: NSSlider allocation and initialization is safe on main thread with valid frame
    let slider: Retained<NSSlider> =
        unsafe { msg_send_id![mtm.alloc::<NSSlider>(), initWithFrame: frame] };

    // SAFETY: Standard NSSlider configuration calls
    unsafe {
        slider.setMinValue(min_value);
        slider.setMaxValue(max_value);
        slider.setDoubleValue(current_value);
        let _: () = msg_send![&slider, setContinuous: true];
        let _: () = msg_send![&slider, setTarget: delegate];
        let _: () = msg_send![&slider, setAction: action];
    }

    slider
}

/// Create a segmented control with the given segment labels.
pub(crate) fn create_segmented_control(
    mtm: MainThreadMarker,
    frame: NSRect,
    labels: &[&str],
    selected_segment: isize,
    delegate: &SettingsActionDelegate,
    action: objc2::runtime::Sel,
) -> Retained<NSSegmentedControl> {
    // SAFETY: NSSegmentedControl allocation and initialization is safe on main thread
    let control: Retained<NSSegmentedControl> =
        unsafe { msg_send_id![mtm.alloc::<NSSegmentedControl>(), initWithFrame: frame] };

    // SAFETY: Standard NSSegmentedControl configuration calls
    unsafe {
        control.setSegmentCount(labels.len() as isize);

        for (i, label) in labels.iter().enumerate() {
            control.setLabel_forSegment(&NSString::from_str(label), i as isize);
            // Set equal width for each segment
            let segment_width = frame.size.width / labels.len() as CGFloat;
            control.setWidth_forSegment(segment_width, i as isize);
        }

        control.setSelectedSegment(selected_segment);
        let _: () = msg_send![&control, setTarget: delegate];
        let _: () = msg_send![&control, setAction: action];
    }

    control
}

/// Create a horizontal separator line.
pub(crate) fn create_separator(
    mtm: MainThreadMarker,
    y: CGFloat,
    width: CGFloat,
) -> Retained<NSBox> {
    let frame = NSRect::new(NSPoint::new(20.0, y), NSSize::new(width - 40.0, 1.0));

    // SAFETY: NSBox allocation and initialization is safe on main thread with valid frame
    let separator: Retained<NSBox> =
        unsafe { msg_send_id![mtm.alloc::<NSBox>(), initWithFrame: frame] };

    // SAFETY: Standard NSBox configuration for separator style
    unsafe {
        separator.setBoxType(NSBoxType::NSBoxSeparator);
    }

    separator
}

/// Create a value label (smaller, centered text for displaying current values).
pub(crate) fn create_value_label(
    mtm: MainThreadMarker,
    frame: NSRect,
    text: &str,
) -> Retained<NSTextField> {
    // SAFETY: NSTextField allocation and initialization is safe on main thread with valid frame
    let label: Retained<NSTextField> =
        unsafe { msg_send_id![mtm.alloc::<NSTextField>(), initWithFrame: frame] };

    // SAFETY: Standard NSTextField configuration calls
    unsafe {
        label.setEditable(false);
        label.setSelectable(false);
        label.setBordered(false);
        label.setDrawsBackground(false);
        label.setStringValue(&NSString::from_str(text));
        let _: () = msg_send![&label, setAlignment: 1_isize]; // NSTextAlignmentCenter

        let font = NSFont::systemFontOfSize(11.0);
        label.setFont(Some(&font));
    }

    label
}

/// Create a tab view for organizing settings into tabs.
pub(crate) fn create_tab_view(mtm: MainThreadMarker, frame: NSRect) -> Retained<NSTabView> {
    // SAFETY: NSTabView allocation and initialization is safe on main thread with valid frame
    let tab_view: Retained<NSTabView> =
        unsafe { msg_send_id![mtm.alloc::<NSTabView>(), initWithFrame: frame] };

    tab_view
}

/// Create a tab view item with the given label.
pub(crate) fn create_tab_item(mtm: MainThreadMarker, label: &str) -> Retained<NSTabViewItem> {
    let identifier = NSString::from_str(label);

    // SAFETY: NSTabViewItem allocation and initialization is safe on main thread
    let tab_item: Retained<NSTabViewItem> =
        unsafe { msg_send_id![mtm.alloc::<NSTabViewItem>(), initWithIdentifier: &*identifier] };

    // SAFETY: Setting label on a valid NSTabViewItem
    unsafe {
        tab_item.setLabel(&NSString::from_str(label));
    }

    tab_item
}
