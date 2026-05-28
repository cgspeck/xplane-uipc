#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    fs::OpenOptions,
    thread,
};

use crate::menu::build_menu;
use ipc_host::{
    IpcCommands, create_ipc_window_and_run,
    value_table::{Entry, Value, create_table_with_entries, set_value_table},
};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{Registry, fmt, layer::SubscriberExt, reload, util::SubscriberInitExt};
pub mod about_window;
mod fsuipc_offsets;
pub mod menu;
mod plugin_state;

use plugin_state::{PluginState, ResolvedMapping};

pub struct PluginStatePtr(*mut std::ffi::c_void);

unsafe impl Send for PluginStatePtr {}
unsafe impl Sync for PluginStatePtr {}

const PLUGIN_NAME: &str = "X-Plane UIPC\0";
const PLUGIN_SIG: &str = "x-plane-uipc\0";
const PLUGIN_DESC: &str = "Provides a local FSUIPC-compatible interface\0";

#[tracing::instrument]
fn plugin_version() -> String {
    // TODO: replace cargo_version with VERGEN_GIT_DESCRIBE once release-please is running
    let cargo_version = env!("CARGO_PKG_VERSION");
    let git_short_sha = match option_env!("VERGEN_GIT_SHA") {
        Some(sha) => &sha[..7],
        None => "unknown",
    };
    let build_date = match option_env!("VERGEN_BUILD_DATE") {
        Some(date) => date,
        None => "unknown",
    };
    let is_dirty = match option_env!("VERGEN_GIT_IS_DIRTY") {
        Some("true") => "dirty",
        Some("false") => "clean",
        _ => "unknown",
    };
    format!(
        "{} (built on {}, git: {}, {})",
        cargo_version, build_date, git_short_sha, is_dirty
    )
}

#[tracing::instrument]
fn about_string() -> String {
    format!(
        "{} v{}",
        PLUGIN_NAME.strip_suffix('\0').unwrap(),
        plugin_version()
    )
}

#[tracing::instrument(skip_all)]
pub fn xplane_log(msg: &str) {
    use std::ffi::CString;
    if let Ok(cs) = CString::new(format!("[xplane-uipc] {}\n", msg)) {
        unsafe {
            XPLMDebugString(cs.as_ptr());
        }
    }
}

#[tracing::instrument]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn XPluginStart(
    out_name: *mut c_char,
    out_sig: *mut c_char,
    out_desc: *mut c_char,
) -> c_int {
    unsafe { XPLMEnableFeature(c"XPLM_USE_NATIVE_PATHS".as_ptr(), 1) };
    unsafe { XPLMEnableFeature(c"XPLM_USE_NATIVE_WIDGET_WINDOWS".as_ptr(), 1) };
    let copy = |s: &str, d: *mut c_char| unsafe {
        std::ptr::copy_nonoverlapping(s.as_ptr() as *const c_char, d, s.len());
    };
    copy(PLUGIN_NAME, out_name);
    copy(PLUGIN_SIG, out_sig);
    copy(PLUGIN_DESC, out_desc);

    xplane_log(&format!("XPluginStart v{}", plugin_version()));

    let mut system_path_buf = [0u8; 512];
    XPLMGetSystemPath(system_path_buf.as_mut_ptr() as *mut c_char);
    let system_path = CStr::from_ptr(system_path_buf.as_ptr() as *const c_char)
        .to_string_lossy()
        .into_owned();
    // [xplane-uipc] XPLMGetSystemPath: C:\X-Plane 12/
    xplane_log(&format!("XPLMGetSystemPath: {}", system_path));

    let mut prefs_path_buf = [0u8; 512];
    XPLMGetPrefsPath(prefs_path_buf.as_mut_ptr() as *mut c_char);
    let prefs_path = CStr::from_ptr(prefs_path_buf.as_ptr() as *const c_char)
        .to_string_lossy()
        .into_owned();
    // [xplane-uipc] XPLMGetPrefsPath: C:\X-Plane 12/Output/preferences/Set X-Plane.prf
    xplane_log(&format!("XPLMGetPrefsPath: {}", prefs_path));

    let log_path = format!("{}uipc.log", system_path);
    xplane_log(&format!("Log path: {}", log_path));

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .expect("Failed to open log file");
    let file_arc = std::sync::Arc::new(std::sync::Mutex::new(file));
    let file_writer = SharedFileWriter {
        inner: file_arc.clone(),
    };
    let file_layer = fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true);
    let (filter_layer, reload_handle) = reload::Layer::new(LevelFilter::INFO);
    let _ = TRACING_FILTER_HANDLE.set(reload_handle);

    let _ = LOG_CONTROLLER.set(LogController {
        file: file_arc,
        log_path: log_path.clone(),
    });

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(file_layer)
        .init();
    tracing::info!("Tracing initialized, log file: {}", log_path);

    // ── Build menu ────────────────────────────────────────────────────────────
    build_menu();

    xplane_log("XPluginStart complete");
    1
}

#[derive(Clone)]
struct SharedFileWriter {
    inner: std::sync::Arc<std::sync::Mutex<std::fs::File>>,
}

impl<'a> tracing_subscriber::fmt::writer::MakeWriter<'a> for SharedFileWriter {
    type Writer = SharedFileGuard;

    fn make_writer(&self) -> Self::Writer {
        SharedFileGuard {
            inner: self.inner.clone(),
        }
    }
}

struct SharedFileGuard {
    inner: std::sync::Arc<std::sync::Mutex<std::fs::File>>,
}

impl std::io::Write for SharedFileGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.lock().unwrap().flush()
    }
}

struct LogController {
    file: std::sync::Arc<std::sync::Mutex<std::fs::File>>,
    log_path: String,
}

pub fn clear_log_file() {
    if let Some(controller) = LOG_CONTROLLER.get() {
        let mut file = controller.file.lock().unwrap();
        let _ = file.flush();
        let new_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&controller.log_path)
            .expect("Failed to reopen log file for clearing");
        *file = new_file;
        use std::io::Write;
        let _ = writeln!(file, "Log file cleared");
    }
    if let Some(tx) = IPC_COMMAND_CHANNEL.lock().unwrap().as_ref() {
        let _ = tx.send(ipc_host::IpcCommands::ResetWarnings);
    }
}

static UIPC_THREAD: std::sync::LazyLock<std::sync::Mutex<Option<thread::JoinHandle<()>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

static IPC_COMMAND_CHANNEL: std::sync::LazyLock<
    std::sync::Mutex<Option<std::sync::mpsc::Sender<IpcCommands>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

static PLUGIN_STATE_PTR: std::sync::LazyLock<std::sync::Mutex<PluginStatePtr>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(PluginStatePtr(std::ptr::null_mut())));

static WRITE_REQUEST_RX: std::sync::LazyLock<
    std::sync::Mutex<Option<std::sync::mpsc::Receiver<ipc_host::WriteRequest>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

static FLIGHT_LOOP_ID: std::sync::LazyLock<std::sync::Mutex<Option<std::ffi::c_void>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

static TRACING_FILTER_HANDLE: std::sync::OnceLock<reload::Handle<LevelFilter, Registry>> =
    std::sync::OnceLock::new();

static LOG_CONTROLLER: std::sync::OnceLock<LogController> = std::sync::OnceLock::new();

#[derive(serde::Deserialize)]
struct TraceConfig {
    settings: Option<TraceSettings>,
}

#[derive(serde::Deserialize)]
struct TraceSettings {
    log_level: Option<String>,
}

fn reload_config_and_apply(config_path: &str) {
    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let config: TraceConfig = match toml::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to parse config.toml: {}. Falling back to INFO.", e);
            if let Some(handle) = TRACING_FILTER_HANDLE.get() {
                let _ = handle.reload(LevelFilter::INFO);
            }
            return;
        }
    };

    let level_str = config
        .settings
        .and_then(|s| s.log_level)
        .unwrap_or_else(|| "info".to_string());

    let level: LevelFilter = match level_str.parse() {
        Ok(l) => l,
        Err(_) => {
            tracing::warn!(
                "Invalid log_level '{}' in config.toml. Falling back to INFO.",
                level_str
            );
            LevelFilter::INFO
        }
    };

    if let Some(handle) = TRACING_FILTER_HANDLE.get() {
        if let Err(e) = handle.reload(level) {
            tracing::warn!("Failed to reload tracing filter: {}", e);
        }
    }
}

/// X-Plane SDK: return value is the interval until the next call in seconds.
/// Positive = seconds, negative = flight loops, 0 = unregister.
const FLIGHT_LOOP_INTERVAL: f32 = 1.0 / 20.0; // 20 Hz

#[unsafe(no_mangle)]
unsafe extern "C" fn flight_loop_callback(
    _inElapsedTimeSinceLastFlightLoop: f32,
    _inElapsedTimeSinceLastCall: f32,
    _inCounter: i32,
    _inRefcon: *mut std::ffi::c_void,
) -> f32 {
    let guard = PLUGIN_STATE_PTR.lock().unwrap();
    let PluginStatePtr(ptr) = *guard;
    if !ptr.is_null() {
        let state = &mut *(ptr as *mut PluginState);

        let write_guard = WRITE_REQUEST_RX.lock().unwrap();
        if let Some(rx) = write_guard.as_ref() {
            while let Ok(write_req) = rx.try_recv() {
                state.write_offset(write_req.offset, write_req.value, write_req.size);
            }
        }

        state.update();
    }
    FLIGHT_LOOP_INTERVAL
}

pub fn load_mappings_and_init() -> Result<(), String> {
    let system_path = get_system_path();
    let mappings_path = format!("{}Resources/plugins/xplane-uipc/mappings.toml", system_path);
    let config_path = format!("{}Resources/plugins/xplane-uipc/config.toml", system_path);
    tracing::info!("mappings_path: {}", mappings_path);
    tracing::info!("config_path: {}", config_path);

    let mapping_config = uipc_mapping::load_mappings(&mappings_path)
        .map_err(|e| format!("Failed to load mappings: {}", e))?;

    if !mapping_config.load_errors.is_empty() {
        tracing::error!(
            "Loaded {} mappings with {} errors from {}",
            mapping_config.mappings.len(),
            mapping_config.load_errors.len(),
            mappings_path
        );
        for err in &mapping_config.load_errors {
            tracing::error!("  {}", err);
        }
    } else {
        tracing::info!(
            "Loaded {} mappings from {}",
            mapping_config.mappings.len(),
            mappings_path
        );
    }

    let resolved_mappings: Vec<ResolvedMapping> = mapping_config
        .mappings
        .into_iter()
        .map(ResolvedMapping::new)
        .collect();

    let mut guard = PLUGIN_STATE_PTR.lock().unwrap();
    let PluginStatePtr(ptr) = *guard;

    if !ptr.is_null() {
        let state = unsafe { &mut *(ptr as *mut PluginState) };
        state.mappings = resolved_mappings;
        tracing::info!("Mappings reloaded successfully");
    } else {
        let state = Box::new(PluginState::new(
            resolved_mappings,
            config_path.to_string(),
            20.0,
        ));
        let new_ptr = Box::into_raw(state) as *mut std::ffi::c_void;
        *guard = PluginStatePtr(new_ptr);
        tracing::info!("Plugin state initialized");
    }

    reload_config_and_apply(&config_path);

    Ok(())
}

fn get_system_path() -> String {
    let mut system_path_buf = [0u8; 512];
    unsafe { XPLMGetSystemPath(system_path_buf.as_mut_ptr() as *mut c_char) };
    let system_path = unsafe {
        CStr::from_ptr(system_path_buf.as_ptr() as *const c_char)
            .to_string_lossy()
            .into_owned()
    };
    system_path
}

#[tracing::instrument]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn XPluginEnable() -> c_int {
    tracing::info!("Enabling plugin...");
    xplane_log("Plugin enabled");

    tracing::info!("Loading mappings and initializing plugin state...");
    if let Err(e) = load_mappings_and_init() {
        tracing::error!("Failed to load mappings: {}", e);
        xplane_log(&format!("Failed to load mappings: {}", e));
    }

    tracing::info!("Pre-populating value table before IPC thread starts...");
    {
        let guard = PLUGIN_STATE_PTR.lock().unwrap();
        let PluginStatePtr(ptr) = *guard;
        if !ptr.is_null() {
            let state = unsafe { &mut *(ptr as *mut PluginState) };
            state.populate_table();
        }
    }

    tracing::info!("Registering flight loop callback...");
    XPLMRegisterFlightLoopCallback(
        Some(flight_loop_callback),
        FLIGHT_LOOP_INTERVAL,
        std::ptr::null_mut(),
    );
    tracing::info!("Flight loop registered at 20Hz");

    tracing::info!("Creating IPC_COMMAND_CHANNEL");
    let (ipc_tx, ipc_rx) = std::sync::mpsc::channel::<IpcCommands>();
    {
        let mut guard = IPC_COMMAND_CHANNEL.lock().unwrap();
        *guard = Some(ipc_tx);
    }

    tracing::info!("Creating write request channel");
    let (write_tx, write_rx) = std::sync::mpsc::channel::<ipc_host::WriteRequest>();
    {
        let mut guard = WRITE_REQUEST_RX.lock().unwrap();
        *guard = Some(write_rx);
    }
    ipc_host::set_write_channel(write_tx);

    let capture_path = format!("{}Resources/plugins/xplane-uipc/capture", get_system_path());

    tracing::info!("Spawning IPC thread");
    let thread_handle = thread::spawn(|| unsafe {
        create_ipc_window_and_run(
            ipc_rx,
            ipc_host::CaptureConfig {
                max: Some(100),
                path: Some(capture_path.into()),
            },
        )
        .expect("Failed to create/run IPC window");
    });

    {
        let mut guard = UIPC_THREAD.lock().unwrap();
        if let Some(old) = guard.take() {
            let _ = old.join();
        }
        *guard = Some(thread_handle);
    }

    1
}

#[tracing::instrument]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn XPluginDisable() {
    tracing::info!("Disabling plugin...");
    xplane_log("Plugin disabled");

    tracing::info!("Unregistering flight loop callback...");
    XPLMUnregisterFlightLoopCallback(Some(flight_loop_callback), std::ptr::null_mut());

    tracing::info!("Cleaning up plugin state...");
    let mut guard = PLUGIN_STATE_PTR.lock().unwrap();
    let PluginStatePtr(ptr) = std::mem::replace(&mut *guard, PluginStatePtr(std::ptr::null_mut()));
    if !ptr.is_null() {
        Box::from_raw(ptr as *mut PluginState);
    }

    let guard = IPC_COMMAND_CHANNEL.lock().unwrap();
    if let Some(tx) = guard.as_ref() {
        tx.send(IpcCommands::Shutdown)
            .expect("Failed to send shutdown command");
    }

    tracing::info!("Cancel command sent, joining thread...");
    let mut guard = UIPC_THREAD.lock().unwrap();
    if let Some(old) = guard.take() {
        let _ = old.join();
    }
    tracing::info!("Thread joined");
}

#[tracing::instrument]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn XPluginStop() {
    tracing::info!("Stopping plugin...");
    xplane_log("XPluginStop complete");
}

#[tracing::instrument(skip_all)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn XPluginReceiveMessage(_from: c_int, _msg: c_int, _param: *mut c_void) {}
