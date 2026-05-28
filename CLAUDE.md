# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

The plugin targets **Windows only** (Win32 shared memory APIs, X-Plane SDK .lib files). Building requires LLVM (for `bindgen`) and the `x86_64-pc-windows-msvc` target.

```shell
# Build (release)
cargo xtask dist          # builds plugin + copies assets to dist/xplane-uipc/

# Deploy to X-Plane
cargo xtask deploy        # dist + copy to C:\X-Plane 12\Resources\plugins\xplane-uipc

# Format / lint / test
cargo fmt
cargo test                # runs tests in uipc-expr, uipc-mapping, ipc_host (value_table, mapped_view, warning)
cargo test -p uipc-expr   # single crate
cargo test -p ipc_host -- test_process_single_read_integer  # single test

# Tools
cargo run -p expr-calculator                    # RPN expression REPL
cargo run -p uipc-debug -- -m xplane_uipc/mappings.toml -s uipc-debug/state.csv --no-ipc  # offline debug TUI

# .NET test client (Windows only, requires .NET 10 SDK + FSUIPCClientDLL)
dotnet build fsuipc-test-client
dotnet test fsuipc-test-client.Tests
```

## Architecture

This is an X-Plane plugin that emulates FSUIPC's shared-memory interface so that FSUIPC client software (Self Loading Cargo, SPAD.neXt, etc.) can read/write X-Plane datarefs as if they were FSUIPC offsets.

### Data flow (runtime)

1. **Mapping load** (`uipc-mapping`): `mappings.toml` is parsed into `DatarefMapping` structs. Each mapping binds an FSUIPC offset to either a simple dataref (with scale/offset_add), an RPN expression over multiple datarefs, or a static value.

2. **Dataref resolution** (`xplane_uipc/plugin_state.rs`): `DatarefMapping` becomes `ResolvedMapping` by calling `XPLMFindDataRef` to get live handles. This happens on plugin enable and on "Reload Mappings".

3. **Flight loop** (20 Hz): `flight_loop_callback` in `lib.rs` calls `PluginState::update()`, which reads each resolved mapping via X-Plane SDK, converts the value to the appropriate FSUIPC type, and writes it into a global `Table` (the value table).

4. **IPC thread** (`ipc_host`): A hidden Win32 window (`UIPCMAIN`) receives `WM_COPYDATA`-style messages from FSUIPC clients. The window proc opens the client's shared file mapping, iterates over FSUIPC request records (sentinel-delimited binary protocol), reads values from the value table for read requests, and sends write requests back to the flight loop thread via an mpsc channel.

### Key boundary: value table

The `Table` in `ipc_host/value_table.rs` is the central shared state. It's a 65536-slot array (one per possible FSUIPC offset) behind `Arc<RwLock<Table>>`. The flight loop thread writes it; the IPC window proc thread reads it. The `active` and `writable` vectors track which offsets are populated/writable.

### Crate roles

- **`xplane_uipc`** (cdylib): Plugin entry points (`XPluginStart/Stop/Enable/Disable`), flight loop, menu, about window. Links against X-Plane SDK via bindgen.
- **`ipc_host`** (cdylib+rlib): Win32 IPC window, FSUIPC binary protocol parsing (`mapped_view.rs`), value table, capture/diagnostic support.
- **`uipc-expr`**: Standalone RPN expression evaluator. No dependencies. Well-tested.
- **`uipc-mapping`**: TOML mapping file loader. Parses `[[mapping]]` entries, validates offsets, resolves array index notation (`dataref[N]`).

### FSUIPC binary protocol

The mapped view contains sequential records: `[reqID:u32][dwOffset:u32][nBytes:u32][sentinel:u32="luaP"][payload:nBytes]`. A zero `reqID` with no subsequent sentinel terminates the stream. Write requests have bit 31 set in `nBytes`. Bad sentinels trigger recovery scanning.

### Thread model

- **Main thread** (X-Plane's): runs `XPluginStart`, `XPluginEnable`, flight loop callback, menu handlers.
- **IPC thread**: spawned in `XPluginEnable`, runs Win32 message pump for `UIPCMAIN` window. Communicates via `IPC_COMMAND_CHANNEL` (commands to IPC) and `WRITE_REQUEST_RX` (write-back to flight loop).

### Configuration

- `config.toml`: runtime settings (`log_level`). Reloaded on mapping reload.
- `mappings.toml`: FSUIPC offset-to-dataref mappings. Hot-reloadable via plugin menu.

## Conventions

- Workspace uses Rust edition 2024, resolver 3.
- `panic = "abort"` in both dev and release profiles (plugin must not unwind into X-Plane).
- `cargo xtask` alias defined in `.cargo/config.toml`.
- X-Plane SDK bindings are generated at build time by `bindgen` from `xplane_sdk.h`; vendored SDK lives in `xplane_uipc/vendor/x-plane-sdk/4.3.0/`.
- Multiple source files `include!()` the generated `bindings.rs` independently (`lib.rs`, `plugin_state.rs`, `menu.rs`, `about_window.rs`).
