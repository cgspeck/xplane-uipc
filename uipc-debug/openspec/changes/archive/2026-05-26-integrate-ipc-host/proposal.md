## Why

The `uipc-debug` tool evaluates FSUIPC mappings from a TOML file and displays computed offset values. It currently has no way to serve those values to FSUIPC clients (e.g., Self Loading Cargo) — it is a read-only formula checker. The `ipc_host` workspace crate provides the Win32 IPC window, shared-memory protocol parsing, and value table infrastructure that the `xplane_uipc` plugin uses to serve offsets. Integrating `ipc_host` into `uipc-debug` lets the tool act as a standalone FSUIPC server: it evaluates mappings, makes the results available to connected clients via the IPC shared memory, and displays the live state in the TUI. The IPC host must also cleanly shut down when the tool quits.

## What Changes

- Add `ipc_host` as a dependency of `uipc-debug`
- IPC mode is enabled by default: spawn a thread running `create_ipc_window_and_run()`, populate the IPC value table from evaluated mappings, and periodically push fresh evaluations to the table
- Add a `--no-ipc` CLI flag to disable IPC mode for purely offline use (existing static CSV behavior)
- Clean shutdown: send `IpcCommands::Shutdown` and join the IPC thread when quitting
- Add a TUI indicator showing IPC vs. offline mode

## Capabilities

### New Capabilities
- `ipc-live-connection`: Run the IPC host to serve evaluated FSUIPC offset values to connected clients; populate the IPC value table from the eval engine at a regular rate
- `ipc-clean-shutdown`: Gracefully shut down the IPC window and thread on quit, including error handling for missing IPC channel

### Modified Capabilities
- (none)

## Impact

- New dependency: `ipc_host` (already in workspace), which transitively adds `windows` and `anyhow` crates
- `main.rs` gains `--no-ipc` CLI flag
- Backwards-compatible: offline mode still available via `--no-ipc`
- Build impact: minor addition of precompiled `windows` crate
