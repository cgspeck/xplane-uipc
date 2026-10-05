## Why

FSUIPC sets aside `0x66C0`–`0x66FF` (64 bytes) as "free for general use, for example in button or keys programming". Clients, and FSUIPC Lua scripts, store their own data there and read it back. Real FSUIPC treats the area as plain memory, so whatever was written is what's read.

xplane-uipc has no such memory. Reads of the area (a client in `mappings reconciliation/uipc.log` reads 8 bytes at `0x66F8`) return zeros and log "not in table" warnings, and writes are refused as writes to unmapped offsets. A client that stores state there loses it on every write.

## What Changes

- Add a 64-byte user area for `0x66C0`–`0x66FF`, held by the IPC window and shared by all clients. Writes of any size starting inside the area are stored byte for byte. Reads starting inside the area return the stored bytes.
- The area starts at zero when the plugin starts. It isn't reset by "Reload Mappings" or "Reset Warnings".
- The area takes precedence over `mappings.toml`. A mapping inside the range is dropped at load with a warning, the same treatment as built-in offsets.
- Writes are logged at `debug`. Neither reads nor writes of the area produce "not in table" or unmapped-write warnings.

Exposing the area to X-Plane as a dataref (so FlyWithLua could act as FSUIPC Lua) is a possible follow-up, and out of scope here.

## Capabilities

### New Capabilities
- `user-area`: FSUIPC's general-use memory at `0x66C0`–`0x66FF`, stored and served by the plugin.

### Modified Capabilities

<!-- None. The area is handled before the value table, so offset-value-semantics and builtin-offsets requirements are unchanged. -->

## Impact

- **`ipc_host/src/mapped_view.rs`**: Read and write paths check the user area before the value table.
- **`ipc_host/src/lib.rs`**: The 64-byte buffer lives in the IPC window's state. This uses the `IpcState` struct proposed in `log-lua-requests`; whichever change is implemented first introduces it.
- **`xplane_uipc/src/plugin_state.rs`**: Mapping loading drops entries inside the user area range, next to `drop_builtin_offsets`.
- **README**: Note the user area next to the built-in `0x337E` offset.
