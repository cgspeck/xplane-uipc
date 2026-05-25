## Why

The IPC host currently discards data when it encounters a malformed mapped view (bad sentinel, unsupported write size) — `process_mapped_view` aborts and the client's message is lost except for a log line. There is no way to inspect what the client actually sent, making it difficult to diagnose protocol mismatches, corruption, or client bugs in production.

## What Changes

- **`create_ipc_window_and_run`** accepts an optional `CaptureConfig` with capture path and max file limit
- **Two new `IpcCommands` variants**: `StartCapture` and `StopCapture` to toggle capture at runtime
- **`process_mapped_view` becomes resilient**: bad sentinels no longer abort — it scans forward for the next valid record and continues
- **`process_mapped_view` returns error count**: `wnd_proc` uses this to decide whether to capture
- **Error-triggered capture**: when capturing is enabled and errors are detected, raw bytes of the mapped view are saved to `<capture_path>/<timestamp>.bin`
- **New capture-inspect CLI tool**: reads `.bin` files, parses records, and displays them for analysis

## Capabilities

### New Capabilities
- `error-capture`: Capturing raw bytes of IPC mapped views that fail processing, for post-hoc analysis
- `capture-inspect`: CLI tool to read and display captured `.bin` files with record-level detail

### Modified Capabilities
*(none — no existing spec-level behavior changes)*

## Impact

- **New dependency**: `chrono` for ISO timestamp formatting in filenames
- **Modified files**:
  - `ipc_host/src/lib.rs` — new capture state, commands, write logic; dead import cleanup
  - `ipc_host/src/mapped_view.rs` — resilient parsing, error count return
  - `xplane_uipc/src/lib.rs` — update `create_ipc_window_and_run` call
  - `ipc_host/examples/run-server.rs`, `run-server-update.rs` — update calls
- **New files**:
  - `ipc_host/src/capture.rs` — `CaptureConfig`, `CaptureState`, helpers
  - `ipc_host/examples/capture-inspect.rs` — analysis CLI
- **No breaking changes** — `capture_path: None, max_captures: None` preserves existing behavior
