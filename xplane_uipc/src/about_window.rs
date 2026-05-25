#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::sync::atomic::AtomicPtr;

use crate::{PLUGIN_NAME, about_string, xplane_log};

static G_WINDOW: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

pub fn about_window_menu_handler() {
    if G_WINDOW
        .load(std::sync::atomic::Ordering::Relaxed)
        .is_null()
    {
        // create the window
        xplane_log("Creating about window");
        let ptr = create_window();
        xplane_log("Storing pointer");
        G_WINDOW.store(ptr, std::sync::atomic::Ordering::SeqCst);
        unsafe { XPShowWidget(ptr) };
        return;
    }

    unsafe {
        let ptr = G_WINDOW.load(std::sync::atomic::Ordering::SeqCst);
        if XPIsWidgetVisible(ptr) == 0 {
            XPShowWidget(ptr);
        } else {
            XPHideWidget(ptr);
        }
    }
}

fn create_window() -> *mut std::ffi::c_void {
    // --- Window dimensions (screen-relative pixels) ---
    // X-Plane widget coordinates: origin (0,0) is BOTTOM-LEFT of screen.
    // Rectangles are specified as  left, top, right, bottom.
    let winLeft = 200;
    let winTop = 600;
    let winRight = 700;
    let winBottom = 500;

    xplane_log("Creating main window widget");
    let window_ptr = unsafe {
        XPCreateWidget(
            winLeft,
            winTop,
            winRight,
            winBottom,
            1,                                 // visible at creation
            PLUGIN_NAME.as_ptr() as *const i8, // window title (shown in title bar)
            1,                                 // this IS the root widget
            std::ptr::null_mut(),              // no parent
            xpWidgetClass_MainWindow,          // standard draggable window
        )
    };

    xplane_log("Setting window properties");
    unsafe {
        XPSetWidgetProperty(window_ptr, xpProperty_MainWindowHasCloseBoxes, 1);
        XPSetWidgetProperty(
            window_ptr,
            xpProperty_MainWindowType,
            xpMainWindowStyle_Translucent.try_into().unwrap(),
        );
    }

    let labelLeft = winLeft + 10;
    let labelTop = winTop - 30; // drop below the title bar
    let labelRight = winRight - 10;
    let labelBottom = winTop - 50;

    let about_str = about_string();
    xplane_log(&about_str);

    let gLabel = unsafe {
        XPCreateWidget(
            labelLeft,
            labelTop,
            labelRight,
            labelBottom,
            1,                               // visible
            about_str.as_ptr() as *const i8, // the label text
            0,                               // NOT a root widget (it has a parent)
            window_ptr,                      // parent = the main window
            xpWidgetClass_Caption,           // standard label / caption widget
        )
    };

    // centre the text within the caption widget
    unsafe { XPSetWidgetProperty(gLabel, xpProperty_CaptionLit, 1) };

    // intercept 'X' button clicks on the window's close box so we can hide rather than destroy the window
    unsafe { XPAddWidgetCallback(window_ptr, Some(widget_handler)) };
    window_ptr
}

unsafe extern "C" fn widget_handler(
    inMessage: XPWidgetMessage,
    inWidget: XPWidgetID,
    _inParam1: isize,
    _inParam2: isize,
) -> i32 {
    // hide window on 'X' press
    if inMessage == xpMessage_CloseButtonPushed {
        let ptr = G_WINDOW.load(std::sync::atomic::Ordering::SeqCst);
        if inWidget == ptr {
            unsafe { XPHideWidget(ptr) };
            return 1;
        }
    }

    return 0;
}
