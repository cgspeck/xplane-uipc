## 1. Setup

- [x] 1.1 Add `chrono` dependency to `ipc_host/Cargo.toml`
- [x] 1.2 Create `ipc_host/src/capture.rs` module with `CaptureConfig`, `CaptureState`
- [x] 1.3 Register `pub mod capture;` in `ipc_host/src/lib.rs`

## 2. Capture state and commands

- [x] 2.1 Add `CaptureConfig` struct with `path: Option<PathBuf>`, `max: Option<usize>`
- [x] 2.2 Add `CaptureState` struct with `enabled: bool`, `path: PathBuf`, `count: usize`, `max: usize`
- [x] 2.3 Add `CAPTURE_STATE: LazyLock<Mutex<Option<CaptureState>>>` global static in `capture.rs`
- [x] 2.4 Add `StartCapture` and `StopCapture` variants to `IpcCommands` enum in `lib.rs`
- [x] 2.5 Handle `StartCapture`/`StopCapture` in the message pump loop (set `enabled` on shared state)

## 3. Resilient parser

- [x] 3.1 Change `process_mapped_view` return type from `()` to `usize` (error count)
- [x] 3.2 On bad sentinel: increment error count, log partial header (reqID, dwOffset, nBytes), scan forward via `find_next_record`, continue or break
- [x] 3.3 On unsupported write size: increment error count (alongside existing log)
- [x] 3.4 Extract raw record parsing into `iterate_records` callback function usable by both `process_mapped_view` and the inspect tool

## 4. Capture logic

- [x] 4.1 Add capture initialization in `create_ipc_window_and_run`: create capture directory if missing (log info), store `CaptureState` in `CAPTURE_STATE`
- [x] 4.2 In `wnd_proc`: determine view size via `VirtualQuery` on the mapped view address
- [x] 4.3 In `wnd_proc`: copy raw bytes into `Vec<u8>` before calling `process_mapped_view`
- [x] 4.4 In `wnd_proc`: if `error_count > 0 && enabled && count < max && !bytes.is_empty()`, write file via `thread::spawn(move || fs::write(...))`
- [x] 4.5 Implement timestamp formatting with `chrono::Local::now().format("%Y-%m-%dT%H-%M-%S.%3fZ")`
- [x] 4.6 Implement filename conflict counter: check if path exists, append `_1`, `_2`, etc.
- [x] 4.7 Implement guardrail: after successful write, if `count >= max`, log warning and set `enabled = false`

## 5. Caller updates

- [x] 5.1 Update `xplane_uipc/src/lib.rs` — pass `CaptureConfig { path: None, max: None }` to `create_ipc_window_and_run`
- [x] 5.2 Update `ipc_host/examples/run-server.rs` — pass `CaptureConfig::none()`
- [x] 5.3 Update `ipc_host/examples/run-server-update.rs` — pass `CaptureConfig::none()`
- [x] 5.4 Add `MENU_START_CAPTURE: usize = 3` and `MENU_STOP_CAPTURE: usize = 4` constants to `xplane_uipc/src/menu.rs`; add handler arms that log an info trace and send `IpcCommands::StartCapture`/`StopCapture` via `IPC_COMMAND_CHANNEL`
- [x] 5.5 Add "Start Capture" and "Stop Capture" menu items in `build_menu()` via `XPLMAppendMenuItem`

## 6. Inspect tool

- [x] 6.1 Create `ipc_host/examples/capture-inspect.rs` binary
- [x] 6.2 Accept `.bin` file paths as CLI args, read each file's bytes
- [x] 6.3 Reuse `iterate_records` to display record number, reqID, offset, read/write, size, sentinel status
- [x] 6.4 Display gap info when bad sentinel is found and recovery position
- [x] 6.5 Exit with code 1 if any file had corrupted records, code 0 otherwise

## 7. Cleanup

- [x] 7.1 Remove dead `GetFileSize`/`GetFileSizeEx` imports from `lib.rs:12`
- [x] 7.2 Run `cargo fmt`

## 8. Verification

- [x] 8.1 `cargo test` — all existing tests pass
- [x] 8.2 `cargo build` — clean compile
- [x] 8.3 `cargo xtask dist` — distribution builds succeed
