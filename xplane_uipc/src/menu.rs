#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::{CString, c_void};

use crate::{about_window::about_window_menu_handler, clear_log_file, xplane_log};

const MENU_ABOUT: usize = 0;
const MENU_RELOAD: usize = 1;
const MENU_CLEAR_LOG: usize = 2;
const MENU_START_CAPTURE: usize = 3;
const MENU_STOP_CAPTURE: usize = 4;

pub unsafe extern "C" fn menu_handler(_menu_ref: *mut c_void, item_ref: *mut c_void) {
    match item_ref as usize {
        MENU_ABOUT => {
            about_window_menu_handler();
            xplane_log("After menu handler");
        }
        MENU_RELOAD => {
            xplane_log("Reload mappings requested");
            if let Err(e) = crate::load_and_resolve_mappings() {
                xplane_log(&format!("Failed to reload mappings: {}", e));
            }
        }
        MENU_CLEAR_LOG => {
            xplane_log("Clear trace log requested");
            clear_log_file();
        }
        MENU_START_CAPTURE => {
            xplane_log("Start capture requested");
            tracing::info!("Start capture requested");
            if let Some(tx) = crate::IPC_COMMAND_CHANNEL.lock().unwrap().as_ref() {
                let _ = tx.send(ipc_host::IpcCommands::StartCapture);
            }
        }
        MENU_STOP_CAPTURE => {
            xplane_log("Stop capture requested");
            tracing::info!("Stop capture requested");
            if let Some(tx) = crate::IPC_COMMAND_CHANNEL.lock().unwrap().as_ref() {
                let _ = tx.send(ipc_host::IpcCommands::StopCapture);
            }
        }
        _ => {}
    }
}

pub fn build_menu() {
    unsafe {
        let plugins_menu = XPLMFindPluginsMenu();
        let menu_name_cs = CString::new("X-Plane UIPC").unwrap();
        let parent_item =
            XPLMAppendMenuItem(plugins_menu, menu_name_cs.as_ptr(), std::ptr::null_mut(), 0);
        let menu_id = XPLMCreateMenu(
            menu_name_cs.as_ptr(),
            plugins_menu,
            parent_item,
            Some(menu_handler),
            std::ptr::null_mut(),
        );
        XPLMAppendMenuItem(
            menu_id,
            CString::new("Reload Mappings").unwrap().as_ptr(),
            MENU_RELOAD as *mut c_void,
            0,
        );
        XPLMAppendMenuItem(
            menu_id,
            CString::new("Clear Trace Log").unwrap().as_ptr(),
            MENU_CLEAR_LOG as *mut c_void,
            0,
        );
        XPLMAppendMenuItem(
            menu_id,
            CString::new("Start Capture").unwrap().as_ptr(),
            MENU_START_CAPTURE as *mut c_void,
            0,
        );
        XPLMAppendMenuItem(
            menu_id,
            CString::new("Stop Capture").unwrap().as_ptr(),
            MENU_STOP_CAPTURE as *mut c_void,
            0,
        );
        XPLMAppendMenuItem(
            menu_id,
            CString::new("About").unwrap().as_ptr(),
            MENU_ABOUT as *mut c_void,
            0,
        );
    }
}
