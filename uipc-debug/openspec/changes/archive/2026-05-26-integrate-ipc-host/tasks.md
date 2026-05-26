## 1. Dependency and CLI setup

- [x] 1.1 Add `ipc_host = { path = "../ipc_host" }` to `Cargo.toml` dependencies
- [x] 1.2 Add `anyhow` to `Cargo.toml` dependencies (from `ipc_host`'s public API)
- [x] 1.3 Add `--no-ipc` boolean flag to `Cli` struct in `main.rs` (default: false, IPC enabled)

## 2. IPC thread lifecycle

- [x] 2.1 Add `ipc_handle: Option<JoinHandle<()>>` and `ipc_tx: Option<Sender<IpcCommands>>` fields to `App` struct
- [x] 2.2 Add `ipc_enabled: bool` field to `App` struct for mode tracking
- [x] 2.3 Create a helper that spawns the IPC thread: builds channel, sets write channel via `ipc_host::set_write_channel()`, spawns thread with `create_ipc_window_and_run`, returns handle + sender
- [x] 2.4 In `main.rs`: when `--no-ipc` is NOT set, spawn IPC thread after loading mappings and pass handle/sender to `App::new()`; if IPC thread fails to start, log warning and continue in offline mode
- [x] 2.5 In `handle_normal_input`: on `q` press, send `IpcCommands::Shutdown` and join the IPC thread before setting `should_quit = true`
- [x] 2.6 Handle channel closed / missing sender gracefully (no panic on `try_send` to closed channel)

## 3. Value table population

- [x] 3.1 Add a method to `App` that takes the current `MappingResult[]` and populates the IPC value table via `set_value_table()` + `create_table_with_entries()`, matching the type conversion logic from `xplane_uipc::PluginState::update()`
- [x] 3.2 Call the table population method after the initial `reload_eval()` in IPC mode
- [x] 3.3 In the TUI event loop, call the table population method on every tick (~50ms) during IPC mode, so the value table stays in sync with state changes
- [x] 3.4 Handle the case where `MappingResult::fsuipc_value` is `None` — skip those entries in the table (mappings with missing state keys)

## 4. TUI mode indicator

- [x] 4.1 In `render_table`: when `app.ipc_enabled` is true, show title ` Mappings (IPC) ` with green color styling
- [x] 4.2 In offline mode (`--no-ipc`), show title ` Mappings (Offline) ` with default styling
