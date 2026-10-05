## Context

From "FSUIPC for Programmers":

- **`0x0D6C`, 4 bytes**: "Parameter associated with any Macro or Lua call sent to the following offset (0D70)".
- **`0x0D70`, 40 bytes**: "Write here the complete identity string of a Macro control or Lua program control in order to have FSUIPC execute it." The text is either `<macro file>:<macro name>` or `<Lua control> <program>`. The Lua controls are `Lua`, `LuaDebug`, `LuaKill`, `LuaSet`, `LuaClear` and `LuaToggle`. "If a parameter is to be supplied, it should first be written to offset 0D6C … Otherwise whatever was last written there will be supplied."

A client log (`mappings reconciliation/uipc.log`) shows writes to both offsets, followed by a read of `0x66F8` in FSUIPC's general-use area. That pattern suggests the client runs a Lua script and expects it to leave a result there. That's an inference, and logging the request text is how we confirm it.

The `0x8001` application key write is already handled the same way: `ipc_host` handles it before the table lookup and logs it at a runtime-selected level. That level is a static `AtomicU8`, set from `[settings] key_write_log_level` through `ipc_host::set_key_write_log_level`.

Per-message state today: the window's `GWLP_USERDATA` holds a `*mut WarnedSet`, which `process_mapped_view` borrows mutably.

## Goals / Non-Goals

**Goals:**
- Stop warning about these writes, and log what was requested, with its parameter, at a level the user picks.
- Group per-feature log levels in one place in `config.toml`, without breaking existing files.

**Non-Goals:**
- Running macros or Lua, or turning requests into X-Plane commands. That may follow once the logs show what clients ask for.
- Answering reads of these offsets. Reads keep being zero-filled.

## Decisions

1. **`0x0D6C` stores the parameter; `0x0D70` logs.** A write to `0x0D6C` (1–4 bytes, little-endian, zero-extended) stores `lua_param`. A write to `0x0D70` (1–40 bytes) logs the request:

   ```
   Lua/macro request "LuaSet slc_doors" (param 3), not supported
   Lua/macro request "LuaKill slc_doors" (no param), not supported
   ```

   The text is cut at the first NUL, and unprintable bytes are escaped, as for the key write. The parameter isn't cleared after use, matching the guide's "whatever was last written there". Neither write is forwarded, counted as rejected, or warned about as unmapped. Writes larger than the documented sizes are handled the same, using only the documented bytes.

2. **State lives in `IpcState`, owned by the IPC window.** `struct IpcState { warned: WarnedSet, lua_param: Option<u32> }` replaces the `WarnedSet` box in `GWLP_USERDATA`. The window procedure is the only thing that uses it, on the IPC thread, so it needs no locking. `ResetWarnings` clears `warned` only. The planned `add-user-area` change will put its 64-byte buffer in the same struct.
   - *Alternative:* another static atomic, like the key-write level. Rejected: it's message state, not configuration, and the user area needs a home anyway.

3. **`[log_levels]` table.**

   ```toml
   [log_levels]
   key_write   = "info"   # 0x8001 application key writes
   lua_request = "info"   # 0x0D6C / 0x0D70 macro and Lua requests
   ```

   Values and parsing are the same as `log_level`: `off`, `error`, `warn`, `info`, `debug`, `trace`. A missing value means `info`, and an invalid one warns and falls back to `info`. `[settings] key_write_log_level` is still read. If both are set, `[log_levels] key_write` wins and a deprecation warning is logged. The global `log_level` still filters on top, as today.

4. **One level helper for both features.** The key-write `AtomicU8`/`LevelFilter` table and its `match level { … }` emitting code become a small `RuntimeLevel` type in `ipc_host` (holding a level, plus a function that logs a message at it). Key writes and Lua requests each have their own instance.

## Risks / Trade-offs

- [Low] **Repeated requests could fill the log at `info`.** A client that sends the same request on every poll would log every time. Set `lua_request = "debug"` or `"off"` if that's noisy. Rate limiting isn't worth adding until it's seen.
- [Low] **Changing the window state type** touches `create_ipc_window`, the window procedure, the reset handler and teardown. Together that's mechanical and covered by the existing IPC tests.

## Open Questions

- Do clients write the parameter in the same message as the request, or in an earlier one? Either works with a stored parameter. Captures will show which.
