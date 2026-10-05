## Why

FSUIPC clients can ask FSUIPC to run a macro or Lua control. They write an optional 4-byte parameter to `0x0D6C`, then the request text to `0x0D70` (for example `LuaSet <program>`). xplane-uipc can't run FSUIPC macros or Lua, and today these writes just produce "Attempt to write non-active offset" warnings. We can't tell what a client wanted, so we can't decide how to support it.

## What Changes

- Accept writes to `0x0D6C` and `0x0D70` as macro/Lua requests instead of writes to unmapped offsets.
- Remember the last parameter written to `0x0D6C`. When `0x0D70` is written, log the request text and that parameter, marked as not supported, at a configurable level.
- Add a `[log_levels]` table to `config.toml` holding the per-feature log levels: `key_write` (the existing `0x8001` key-write level) and `lua_request` (new). `[settings] key_write_log_level` keeps working as a deprecated alias.
- Replace the IPC window's bare `WarnedSet` pointer with an `IpcState` struct that can hold state across messages, starting with the last `0x0D6C` parameter.

Running macros or Lua, and mapping requests to X-Plane commands, are out of scope. The log output is meant to show which requests need supporting.

## Capabilities

### New Capabilities
- `lua-macro-requests`: Handling of FSUIPC macro and Lua execution requests written to `0x0D6C`/`0x0D70`.

### Modified Capabilities
- `fsuipc-ipc-protocol`: The `0x8001` key-write log level moves to `[log_levels] key_write`, with `[settings] key_write_log_level` kept as an alias.

## Impact

- **`ipc_host/src/mapped_view.rs`**: Handle the two offsets in the write path, ahead of the table lookup, the same way as `0x8001`. Add a runtime-selected log level for Lua requests next to the key-write one.
- **`ipc_host/src/lib.rs`**: `IpcState { warned: WarnedSet, lua_param: Option<u32> }` replaces the `WarnedSet` pointer in the window's user data. `process_mapped_view` takes `&mut IpcState`.
- **`xplane_uipc/src/lib.rs`**: Parse `[log_levels]`, keep the `[settings] key_write_log_level` alias, and set both levels on load and reload.
- **`xplane_uipc/config.toml`**, **README**: Document `[log_levels]`.
