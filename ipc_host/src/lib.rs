pub mod capture;
pub mod mapped_view;
pub mod value_table;
mod warning;

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, RwLock};

// use crate::value_table::{VALUE_TABLE, Value};
use windows::Win32::Foundation::*;

use windows::Win32::Foundation::{ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND};
use windows::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, MEMORY_MAPPED_VIEW_ADDRESS};
use windows::Win32::System::{
    DataExchange::GlobalGetAtomNameA,
    Memory::{FILE_MAP_WRITE, MapViewOfFile, OpenFileMappingA, UnmapViewOfFile, VirtualQuery},
};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, DestroyWindow, DispatchMessageW, PM_REMOVE, PeekMessageW, RegisterClassW,
    ShowWindow, TranslateMessage, WNDCLASSW,
};
use windows::{
    Win32::Foundation::*, Win32::System::LibraryLoader::GetModuleHandleW,
    Win32::UI::WindowsAndMessaging::*, core::*,
};

use crate::mapped_view::process_mapped_view;
use crate::value_table::get_value_table;
use crate::warning::WarnedSet;
pub use capture::CaptureConfig;

pub enum IpcCommands {
    ResetWarnings,
    Shutdown,
    StartCapture,
    StopCapture,
}

#[derive(Debug, Clone)]
pub struct WriteRequest {
    pub offset: u16,
    pub value: f64,
    pub size: usize,
}

static WRITE_CHANNEL: std::sync::LazyLock<std::sync::Mutex<Option<Sender<WriteRequest>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

pub fn set_write_channel(tx: Sender<WriteRequest>) {
    let mut guard = WRITE_CHANNEL.lock().unwrap();
    *guard = Some(tx);
}

pub fn try_send_write(offset: u16, value: f64, size: usize) {
    let guard = WRITE_CHANNEL.lock().unwrap();
    if let Some(tx) = guard.as_ref() {
        let _ = tx.send(WriteRequest {
            offset,
            value,
            size,
        });
    }
}

#[tracing::instrument]
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    tracing::debug!("Received message: {}", msg);

    if msg == WM_NCCREATE {
        let cs = &*(lparam.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as _);
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    } else if msg < 0x8000 {
        tracing::debug!(
            "Fall-through to default message handler for message: {}",
            msg
        );
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    tracing::debug!("Message is a registered message (greater than WM_USER)");
    let warned_set = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WarnedSet;
    tracing::trace!(
        "Retrieved warned_set pointer from window user data: {:?}",
        warned_set
    );
    if warned_set.is_null() {
        tracing::error!("warned_set pointer is null, this should not happen");
        return LRESULT(0);
    }
    // msg.wparam points to a GlobalAddAtomA, the text of which contains the name of a mapped file
    // We can use GlobalGetAtomNameA to retrieve the name, then OpenFileMappingA and MapViewOfFile to read the contents of the mapped file.
    let mut atom_name = [0u8; 256]; // Buffer to hold the atom name
    let atom_id = wparam.0 as u16; // The atom ID is passed in wparam
    tracing::trace!("Received Atom ID: {}", atom_id);

    let name_len = unsafe { GlobalGetAtomNameA(atom_id, &mut atom_name) };
    if name_len == 0 {
        tracing::trace!("Failed to get atom name for ID: {}", atom_id);
        return LRESULT(1);
    }
    let atom_name_str = std::str::from_utf8(&atom_name[..name_len as usize])
        .unwrap_or("<Invalid UTF-8 in atom name>");
    tracing::trace!("Received Atom Name: {}", atom_name_str);
    // open the file mapping and read the contents
    let handle_res = unsafe {
        OpenFileMappingA(
            FILE_MAP_WRITE.0,
            false,
            PCSTR(atom_name_str.as_ptr() as *const u8),
        )
    };
    if handle_res.is_err() {
        tracing::trace!(
            "Failed to open file mapping for atom name: {}",
            atom_name_str
        );
        return LRESULT(1);
    }
    let handle = handle_res.unwrap();
    if handle.is_invalid() {
        tracing::trace!(
            "Failed to open file mapping for atom name: {}",
            atom_name_str
        );
        return LRESULT(1);
    }
    tracing::trace!(
        "Successfully opened file mapping for atom name: {}",
        atom_name_str
    );

    let mapped_view: MEMORY_MAPPED_VIEW_ADDRESS =
        unsafe { MapViewOfFile(handle, FILE_MAP_WRITE, 0, 0, 0) };

    if mapped_view.Value.is_null() {
        tracing::trace!(
            "Failed to map view of file for atom name: {}",
            atom_name_str
        );
        unsafe { CloseHandle(handle) };
        return LRESULT(0);
    }
    tracing::trace!(
        "Successfully mapped view of file for atom name: {}",
        atom_name_str
    );

    let mapped_view_ptr: *const u8 = mapped_view.Value as *const u8;

    // ── Determine view size via VirtualQuery ──────────────────────────────
    let view_size: usize = {
        let mut mbi = MEMORY_BASIC_INFORMATION::default();
        let result = unsafe {
            VirtualQuery(
                Some(mapped_view_ptr as *const _),
                &mut mbi as *mut MEMORY_BASIC_INFORMATION,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        if result == 0 {
            tracing::error!("VirtualQuery failed, cannot determine view size");
            0
        } else {
            mbi.RegionSize
        }
    };

    // ── Copy raw bytes before processing ──────────────────────────────────
    let raw_bytes = if view_size > 0 {
        unsafe { std::slice::from_raw_parts(mapped_view_ptr, view_size).to_vec() }
    } else {
        Vec::new()
    };

    // ── Process the mapped view ───────────────────────────────────────────
    let table_arc = get_value_table();
    let table = table_arc.read().unwrap();
    tracing::trace!("Aquired table lock");
    tracing::trace!("Calling process_mapped_view");
    let error_count =
        unsafe { process_mapped_view(mapped_view_ptr, view_size, &table, &mut *warned_set) };

    // ── Capture if errors detected ────────────────────────────────────────
    if error_count > 0 {
        let mut guard = capture::CAPTURE_STATE.lock().unwrap();
        if let Some(state) = guard.as_mut() {
            if state.enabled && state.count < state.max && !raw_bytes.is_empty() {
                let ts = chrono::Local::now()
                    .format("%Y-%m-%dT%H-%M-%S.%3fZ")
                    .to_string();
                let mut bin_path = state.path.join(format!("{}.bin", ts));
                let mut counter = 0u32;
                while bin_path.exists() {
                    counter += 1;
                    bin_path = state.path.join(format!("{}_{}.bin", ts, counter));
                }
                let bytes = raw_bytes.clone();
                let path = bin_path.clone();
                let _ = std::thread::spawn(move || {
                    if let Err(e) = std::fs::write(&path, &bytes) {
                        tracing::warn!("Failed to write capture file {:?}: {}", path, e);
                    }
                });
                tracing::info!(
                    "Captured view with {} errors to {:?}",
                    error_count,
                    bin_path
                );
                state.count += 1;
                if state.count >= state.max {
                    tracing::warn!(
                        "Capture guardrail reached ({} files), disabling capture",
                        state.max
                    );
                    state.enabled = false;
                }
            }
        }
    }

    tracing::trace!("Finished processing mapped view, unmapping and closing handle");
    unsafe {
        UnmapViewOfFile(mapped_view);
        CloseHandle(handle);
    }

    LRESULT(1)
}

#[tracing::instrument(skip(warned_set_ptr))]
pub fn create_ipc_window(warned_set_ptr: *mut WarnedSet) -> anyhow::Result<HWND> {
    tracing::info!("Creating IPC Window...");
    unsafe {
        // let instance: HINSTANCE = GetModuleHandleW(None)?;
        let instance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
        // let instance = GetInstance(None)?;
        let class_name = w!("UIPCMAIN"); // The 'Class Name' used for FindWindowW

        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            lpszClassName: class_name,
            ..Default::default()
        };

        tracing::info!("Registering window class...");
        // 1. Register the class with Windows
        if RegisterClassW(&wc) == 0 {
            let last_error = unsafe { GetLastError() };
            if last_error != ERROR_CLASS_ALREADY_EXISTS {
                return Err(anyhow::anyhow!(
                    "Failed to register window class: {}",
                    last_error.0
                ));
            }
        }

        // 2. Create the window instance
        let some_instance = Some(instance);
        tracing::info!("Creating window instance...");
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("UIPCMAIN"), // The 'Window Title' (also works for FindWindowW)
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            some_instance,
            Some(warned_set_ptr as *mut _), // Pass the warned_set pointer as the lpParam to the window, so we can access it in the wnd_proc
        );

        // if hwnd.0 == 0 {
        let unwrapped_hwnd = hwnd.unwrap();

        if unwrapped_hwnd.0 == std::ptr::null_mut() {
            return Err(anyhow::anyhow!("Failed to IPC window"));
        }

        if false {
            // TODO: Show the window (for debugging - we can make this conditional on a debug flag or environment variable)
            tracing::info!("Showing IPC window...");
            unsafe {
                ShowWindow(unwrapped_hwnd, SW_SHOW).ok();
            }
        }

        Ok(unwrapped_hwnd)
    }
}

#[tracing::instrument(skip(config))]
pub unsafe fn create_ipc_window_and_run(
    rx: Receiver<IpcCommands>,
    config: capture::CaptureConfig,
) -> anyhow::Result<()> {
    tracing::info!("Creating IPC window...");
    let warned_set = Box::new(WarnedSet::new());
    let warned_set_ptr = Box::into_raw(warned_set);

    // ── Initialize capture state ───────────────────────────────────────────
    if let Some(ref path) = config.path {
        if !path.exists() {
            tracing::info!("Creating capture directory: {:?}", path);
            std::fs::create_dir_all(path)?;
        }
        let max = config.max.unwrap_or(usize::MAX);
        let mut guard = capture::CAPTURE_STATE.lock().unwrap();
        *guard = Some(capture::CaptureState {
            enabled: false,
            path: path.clone(),
            count: 0,
            max,
        });
    }

    let hwnd = create_ipc_window(warned_set_ptr)?;
    tracing::trace!("HWND created: {:?}", hwnd);

    let hwnd = hwnd.0;
    let mut msg = MSG::default();

    let mut continue_loop = true;

    while continue_loop {
        rx.try_recv().ok().map(|cmd| match cmd {
            IpcCommands::ResetWarnings => {
                tracing::info!("Resetting warnings...");
                unsafe {
                    let warned_set_ptr =
                        GetWindowLongPtrW(HWND(hwnd), GWLP_USERDATA) as *mut WarnedSet;
                    if !warned_set_ptr.is_null() {
                        (&mut *warned_set_ptr).clear_all();
                    }
                }
                crate::mapped_view::reset_logged_sentinels();
            }
            IpcCommands::StartCapture => {
                tracing::info!("Starting capture...");
                let mut guard = capture::CAPTURE_STATE.lock().unwrap();
                if let Some(state) = guard.as_mut() {
                    state.enabled = true;
                }
            }
            IpcCommands::StopCapture => {
                tracing::info!("Stopping capture...");
                let mut guard = capture::CAPTURE_STATE.lock().unwrap();
                if let Some(state) = guard.as_mut() {
                    state.enabled = false;
                }
            }
            IpcCommands::Shutdown => {
                tracing::info!("Shutting down IPC window...");
                unsafe {
                    DestroyWindow(HWND(hwnd)).ok();
                }
                continue_loop = false;
            }
        });

        let ret = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) };

        if ret.as_bool() {
            tracing::debug!("Got message: {:04x}", msg.message);
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    unsafe { Box::from_raw(warned_set_ptr) };
    Ok(())
}
