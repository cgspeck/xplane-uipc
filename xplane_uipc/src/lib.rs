#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use atomic_float::AtomicF32;
use bindings::*;

use std::{
    ffi::{CStr, c_char, c_int, c_void},
    fs::OpenOptions,
    sync::atomic::{AtomicBool, Ordering},
    thread,
};

use crate::menu::build_menu;
use ipc_host::{IpcCommands, create_ipc_window_and_run};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{Registry, fmt, layer::SubscriberExt, reload, util::SubscriberInitExt};
pub mod about_window;
pub mod menu;
mod plugin_state;

use plugin_state::{PluginState, ResolvedMapping, drop_builtin_offsets, drop_user_area_offsets};

pub struct PluginStatePtr(*mut std::ffi::c_void);

unsafe impl Send for PluginStatePtr {}
unsafe impl Sync for PluginStatePtr {}

const PLUGIN_NAME: &str = "X-Plane UIPC\0";
const PLUGIN_SIG: &str = "x-plane-uipc\0";
const PLUGIN_DESC: &str = "Provides a local FSUIPC-compatible interface\0";

static FLIGHT_LOOP_INTERVAL: AtomicF32 = AtomicF32::new(1.0 / 20.0);

static UIPC_THREAD: std::sync::LazyLock<std::sync::Mutex<Option<thread::JoinHandle<()>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

static IPC_COMMAND_CHANNEL: std::sync::LazyLock<
    std::sync::Mutex<Option<std::sync::mpsc::Sender<IpcCommands>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

static DATAREF_RESOLUTION_REQUIRED: AtomicBool = AtomicBool::new(true);

static PLUGIN_STATE_PTR: std::sync::LazyLock<std::sync::Mutex<PluginStatePtr>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(PluginStatePtr(std::ptr::null_mut())));

static WRITE_REQUEST_RX: std::sync::LazyLock<
    std::sync::Mutex<Option<std::sync::mpsc::Receiver<ipc_host::WriteRequest>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

static TRACING_FILTER_HANDLE: std::sync::OnceLock<reload::Handle<LevelFilter, Registry>> =
    std::sync::OnceLock::new();

static LOG_CONTROLLER: std::sync::OnceLock<LogController> = std::sync::OnceLock::new();

#[tracing::instrument]
fn plugin_version() -> String {
    // TODO: replace cargo_version with VERGEN_GIT_DESCRIBE once release-please is running
    let cargo_version = env!("CARGO_PKG_VERSION");
    let git_short_sha = match option_env!("VERGEN_GIT_SHA") {
        Some(sha) => sha.get(..7).unwrap_or(sha),
        None => "unknown",
    };
    let build_date = option_env!("VERGEN_BUILD_DATE").unwrap_or("unknown");
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

/// X-Plane plugin entry point: report the plugin name, signature and description.
///
/// # Safety
///
/// Called by X-Plane on its main thread, as the plugin SDK requires. `out_name`,
/// `out_sig` and `out_desc` must each point to a writable buffer of at least
/// 256 bytes.
#[tracing::instrument]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn XPluginStart(
    out_name: *mut c_char,
    out_sig: *mut c_char,
    out_desc: *mut c_char,
) -> c_int {
    // SAFETY: X-Plane SDK calls with valid pointers provided by the host
    unsafe {
        XPLMEnableFeature(c"XPLM_USE_NATIVE_PATHS".as_ptr(), 1);
        XPLMEnableFeature(c"XPLM_USE_NATIVE_WIDGET_WINDOWS".as_ptr(), 1);
    }
    let copy = |s: &str, d: *mut c_char| unsafe {
        std::ptr::copy_nonoverlapping(s.as_ptr() as *const c_char, d, s.len());
    };
    copy(PLUGIN_NAME, out_name);
    copy(PLUGIN_SIG, out_sig);
    copy(PLUGIN_DESC, out_desc);

    xplane_log(&format!("XPluginStart v{}", plugin_version()));

    let mut system_path_buf = [0u8; 512];
    unsafe { XPLMGetSystemPath(system_path_buf.as_mut_ptr() as *mut c_char) };
    let system_path = unsafe { CStr::from_ptr(system_path_buf.as_ptr() as *const c_char) }
        .to_string_lossy()
        .into_owned();
    // [xplane-uipc] XPLMGetSystemPath: C:\X-Plane 12/
    xplane_log(&format!("XPLMGetSystemPath: {}", system_path));

    let mut prefs_path_buf = [0u8; 512];
    unsafe { XPLMGetPrefsPath(prefs_path_buf.as_mut_ptr() as *mut c_char) };
    let prefs_path = unsafe { CStr::from_ptr(prefs_path_buf.as_ptr() as *const c_char) }
        .to_string_lossy()
        .into_owned();
    // [xplane-uipc] XPLMGetPrefsPath: C:\X-Plane 12/Output/preferences/Set X-Plane.prf
    xplane_log(&format!("XPLMGetPrefsPath: {}", prefs_path));

    let log_path = format!("{}uipc.log", system_path);
    xplane_log(&format!("Log path: {}", log_path));

    // A missing log file must not take down the simulator; log to nowhere instead.
    let file = match open_truncated(&log_path) {
        Ok(f) => Some(f),
        Err(e) => {
            xplane_log(&format!(
                "Failed to open log file {}: {}; file logging disabled",
                log_path, e
            ));
            None
        }
    };
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

fn open_truncated(path: &str) -> std::io::Result<std::fs::File> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
}

type SharedLogFile = std::sync::Arc<std::sync::Mutex<Option<std::fs::File>>>;

#[derive(Clone)]
struct SharedFileWriter {
    inner: SharedLogFile,
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
    inner: SharedLogFile,
}

impl std::io::Write for SharedFileGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.inner.lock().unwrap().as_mut() {
            Some(f) => f.write(buf),
            None => Ok(buf.len()),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self.inner.lock().unwrap().as_mut() {
            Some(f) => f.flush(),
            None => Ok(()),
        }
    }
}

struct LogController {
    file: SharedLogFile,
    log_path: String,
}

pub fn clear_log_file() {
    if let Some(controller) = LOG_CONTROLLER.get() {
        use std::io::Write;
        let mut file = controller.file.lock().unwrap();
        if let Some(f) = file.as_mut() {
            let _ = f.flush();
        }
        match open_truncated(&controller.log_path) {
            Ok(mut new_file) => {
                let _ = writeln!(new_file, "Log file cleared");
                *file = Some(new_file);
            }
            Err(e) => xplane_log(&format!(
                "Failed to reopen log file {} for clearing: {}",
                controller.log_path, e
            )),
        }
    }
    if let Some(tx) = IPC_COMMAND_CHANNEL.lock().unwrap().as_ref() {
        let _ = tx.send(ipc_host::IpcCommands::ResetWarnings);
    }
}

#[derive(serde::Deserialize)]
struct Config {
    settings: Settings,
    #[serde(default)]
    log_levels: LogLevels,
}

#[derive(serde::Deserialize)]
struct Settings {
    update_rate_hz: Option<u8>,
    log_level: Option<String>,
    /// Deprecated: use `[log_levels] key_write`.
    key_write_log_level: Option<String>,
}

/// Levels for messages that have their own setting.
#[derive(serde::Deserialize, Default)]
struct LogLevels {
    key_write: Option<String>,
    lua_request: Option<String>,
}

/// The key-write level setting: `[log_levels] key_write`, falling back to the
/// deprecated `[settings] key_write_log_level`. Warns when both are set.
fn key_write_setting<'a>(new: Option<&'a str>, deprecated: Option<&'a str>) -> Option<&'a str> {
    if new.is_some() && deprecated.is_some() {
        tracing::warn!(
            "config.toml sets both [log_levels] key_write and the deprecated [settings] key_write_log_level; using [log_levels] key_write"
        );
    }
    new.or(deprecated)
}

/// Parse a log level setting: missing means INFO, and an invalid value warns
/// and falls back to INFO.
fn parse_level_setting(name: &str, value: Option<&str>) -> LevelFilter {
    let value = value.unwrap_or("info");
    value.parse().unwrap_or_else(|_| {
        tracing::warn!(
            "Invalid {} '{}' in config.toml. Falling back to INFO.",
            name,
            value
        );
        LevelFilter::INFO
    })
}

fn parse_config_and_apply(config_path: &str) {
    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let config: Config = match toml::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to parse config.toml: {}. Falling back to INFO.", e);
            if let Some(handle) = TRACING_FILTER_HANDLE.get() {
                let _ = handle.reload(LevelFilter::INFO);
            }
            ipc_host::set_key_write_log_level(LevelFilter::INFO);
            ipc_host::set_lua_request_log_level(LevelFilter::INFO);
            return;
        }
    };

    let level = parse_level_setting("log_level", config.settings.log_level.as_deref());

    if let Some(handle) = TRACING_FILTER_HANDLE.get()
        && let Err(e) = handle.reload(level)
    {
        tracing::warn!("Failed to reload tracing filter: {}", e);
    }

    ipc_host::set_key_write_log_level(parse_level_setting(
        "key_write",
        key_write_setting(
            config.log_levels.key_write.as_deref(),
            config.settings.key_write_log_level.as_deref(),
        ),
    ));
    ipc_host::set_lua_request_log_level(parse_level_setting(
        "lua_request",
        config.log_levels.lua_request.as_deref(),
    ));

    if let Some(hz) = config.settings.update_rate_hz {
        if hz == 0 {
            tracing::warn!("Invalid update_rate_hz 0 in config.toml, keeping current rate");
            return;
        }
        tracing::info!("Setting flight update rate to {} hz", hz);
        let v = 1.0 / (hz as f32);
        FLIGHT_LOOP_INTERVAL.store(v, Ordering::Relaxed)
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn flight_loop_callback(
    _elapsed_since_last_flight_loop: f32,
    _elapsed_since_last_call: f32,
    _counter: i32,
    _refcon: *mut std::ffi::c_void,
) -> f32 {
    if DATAREF_RESOLUTION_REQUIRED.load(Ordering::Relaxed) {
        tracing::info!("resolving datarefs during flight loop callback");
        match find_load_and_resolve_mappings() {
            Ok(_) => tracing::info!("resolved datarefs"),
            Err(e) => tracing::error!("error loading mappings or resolving datarefs: {}", e),
        }
        DATAREF_RESOLUTION_REQUIRED.store(false, Ordering::Relaxed);
    }

    let guard = PLUGIN_STATE_PTR.lock().unwrap();
    let PluginStatePtr(ptr) = *guard;
    if !ptr.is_null() {
        let state = unsafe { &mut *(ptr as *mut PluginState) };

        let write_guard = WRITE_REQUEST_RX.lock().unwrap();
        if let Some(rx) = write_guard.as_ref() {
            while let Ok(write_req) = rx.try_recv() {
                state.write_offset(write_req.offset, write_req.value, write_req.size);
            }
        }

        state.update();
    }
    FLIGHT_LOOP_INTERVAL.load(Ordering::Relaxed)
}

pub fn find_and_load_config() -> Result<(), String> {
    let system_path = get_system_path();
    let config_path = format!("{}Resources/plugins/xplane-uipc/config.toml", system_path);
    tracing::info!("config_path: {}", config_path);
    parse_config_and_apply(&config_path);
    Ok(())
}

pub fn find_load_and_resolve_mappings() -> Result<(), String> {
    let system_path = get_system_path();
    let mappings_path = format!("{}Resources/plugins/xplane-uipc/mappings.toml", system_path);
    tracing::info!("mappings_path: {}", mappings_path);

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
    let mappings = drop_user_area_offsets(drop_builtin_offsets(mapping_config.mappings));
    let resolved_mappings: Vec<ResolvedMapping> =
        mappings.into_iter().map(ResolvedMapping::new).collect();

    let mut guard = PLUGIN_STATE_PTR.lock().unwrap();
    let PluginStatePtr(ptr) = *guard;

    if !ptr.is_null() {
        let state = unsafe { &mut *(ptr as *mut PluginState) };
        state.mappings = resolved_mappings;
        tracing::info!("Mappings reloaded successfully");
    } else {
        let state = Box::new(PluginState::new(resolved_mappings));
        let new_ptr = Box::into_raw(state) as *mut std::ffi::c_void;
        *guard = PluginStatePtr(new_ptr);
        tracing::info!("Plugin state initialized");
    }

    Ok(())
}

fn get_system_path() -> String {
    let mut system_path_buf = [0u8; 512];
    unsafe { XPLMGetSystemPath(system_path_buf.as_mut_ptr() as *mut c_char) };

    unsafe {
        CStr::from_ptr(system_path_buf.as_ptr() as *const c_char)
            .to_string_lossy()
            .into_owned()
    }
}

/// X-Plane plugin entry point: the plugin was enabled.
///
/// # Safety
///
/// Called by X-Plane on its main thread, as the plugin SDK requires. Do not
/// call it directly.
#[tracing::instrument]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn XPluginEnable() -> c_int {
    tracing::info!("Enabling plugin...");
    xplane_log("Plugin enabled");

    tracing::info!("Loading config and initializing plugin state...");
    if let Err(e) = find_and_load_config() {
        tracing::error!("Failed to load config: {}", e);
        xplane_log(&format!("Failed to load config: {}", e));
    }

    // Plugin state is freed on disable, so it must be rebuilt on every enable.
    DATAREF_RESOLUTION_REQUIRED.store(true, Ordering::Release);

    tracing::info!("Registering flight loop callback...");
    unsafe {
        XPLMRegisterFlightLoopCallback(
            Some(flight_loop_callback),
            FLIGHT_LOOP_INTERVAL.load(Ordering::Relaxed),
            std::ptr::null_mut(),
        );
    }

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
    let thread_handle = thread::spawn(|| {
        let result = unsafe {
            create_ipc_window_and_run(
                ipc_rx,
                ipc_host::CaptureConfig {
                    max: Some(100),
                    path: Some(capture_path.into()),
                },
            )
        };
        // Panicking here would abort X-Plane (panic = "abort"); report and exit the thread.
        if let Err(e) = result {
            tracing::error!("IPC window failed, FSUIPC clients will not connect: {}", e);
        }
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

/// X-Plane plugin entry point: the plugin was disabled.
///
/// # Safety
///
/// Called by X-Plane on its main thread, as the plugin SDK requires. Do not
/// call it directly.
#[tracing::instrument]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn XPluginDisable() {
    tracing::info!("Disabling plugin...");
    xplane_log("Plugin disabled");

    tracing::info!("Unregistering flight loop callback...");
    unsafe { XPLMUnregisterFlightLoopCallback(Some(flight_loop_callback), std::ptr::null_mut()) };

    tracing::info!("Shutting down IPC thread...");
    {
        // The receiver is gone if the IPC thread already exited; nothing to shut down then.
        if let Some(tx) = IPC_COMMAND_CHANNEL.lock().unwrap().take() {
            let _ = tx.send(IpcCommands::Shutdown);
        }
    }
    {
        let mut guard = UIPC_THREAD.lock().unwrap();
        if let Some(old) = guard.take() {
            let _ = old.join();
        }
    }
    tracing::info!("IPC thread joined");

    tracing::info!("Cleaning up plugin state...");
    let mut guard = PLUGIN_STATE_PTR.lock().unwrap();
    let PluginStatePtr(ptr) = std::mem::replace(&mut *guard, PluginStatePtr(std::ptr::null_mut()));
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr as *mut PluginState)) };
    }
}

/// X-Plane plugin entry point: the plugin is being unloaded.
///
/// # Safety
///
/// Called by X-Plane on its main thread, as the plugin SDK requires. Do not
/// call it directly.
#[tracing::instrument]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn XPluginStop() {
    tracing::info!("Stopping plugin...");
    xplane_log("XPluginStop complete");
}

/// X-Plane plugin entry point: an inter-plugin or sim message arrived.
///
/// # Safety
///
/// Called by X-Plane on its main thread, as the plugin SDK requires. Do not
/// call it directly.
#[tracing::instrument(skip_all)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn XPluginReceiveMessage(_from: c_int, msg: c_int, param: *mut c_void) {
    if msg == XPLM_MSG_PLANE_LOADED {
        let plane_index = param as isize as i32;
        if plane_index == 0 {
            tracing::info!("User aircraft changed, setting DATAREF_RESOLUTION_REQUIRED flag");
            DATAREF_RESOLUTION_REQUIRED.store(true, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_setting_parses_valid_values() {
        assert_eq!(parse_level_setting("x", Some("off")), LevelFilter::OFF);
        assert_eq!(parse_level_setting("x", Some("debug")), LevelFilter::DEBUG);
        assert_eq!(parse_level_setting("x", Some("WARN")), LevelFilter::WARN);
    }

    #[test]
    fn level_setting_defaults_to_info() {
        assert_eq!(parse_level_setting("x", None), LevelFilter::INFO);
        assert_eq!(parse_level_setting("x", Some("loud")), LevelFilter::INFO);
    }

    #[test]
    fn log_levels_table_is_parsed() {
        let config: Config = toml::from_str(
            "[settings]\nlog_level = \"info\"\n[log_levels]\nkey_write = \"off\"\nlua_request = \"debug\"\n",
        )
        .unwrap();
        assert_eq!(config.log_levels.key_write.as_deref(), Some("off"));
        assert_eq!(config.log_levels.lua_request.as_deref(), Some("debug"));
    }

    #[test]
    fn config_without_log_levels_still_parses() {
        let config: Config = toml::from_str("[settings]\nkey_write_log_level = \"off\"\n").unwrap();
        assert!(config.log_levels.key_write.is_none());
        assert!(config.log_levels.lua_request.is_none());
        assert_eq!(config.settings.key_write_log_level.as_deref(), Some("off"));
    }

    #[test]
    fn shipped_config_parses_with_log_levels() {
        let config: Config = toml::from_str(include_str!("../config.toml")).unwrap();
        assert_eq!(config.log_levels.key_write.as_deref(), Some("info"));
        assert_eq!(config.log_levels.lua_request.as_deref(), Some("info"));
        assert!(config.settings.key_write_log_level.is_none());
    }

    #[test]
    fn deprecated_key_write_setting_used_when_new_one_missing() {
        assert_eq!(key_write_setting(None, Some("off")), Some("off"));
        assert_eq!(key_write_setting(None, None), None);
    }

    #[test]
    fn new_key_write_setting_wins_over_deprecated_one() {
        assert_eq!(key_write_setting(Some("info"), Some("off")), Some("info"));
        assert_eq!(key_write_setting(Some("debug"), None), Some("debug"));
    }
}
