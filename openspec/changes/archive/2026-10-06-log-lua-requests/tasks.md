## 1. IPC window state

- [x] 1.1 Add `IpcState { warned: WarnedSet, lua_param: Option<u32> }` in `ipc_host` and store it in `GWLP_USERDATA` instead of the bare `WarnedSet`; update `create_ipc_window`, `wnd_proc`, the `ResetWarnings` handler (clears `warned` only) and teardown
- [x] 1.2 Change `process_mapped_view` to take `&mut IpcState`; update its callers and tests

## 2. Runtime log levels

- [x] 2.1 Extract the key-write `AtomicU8` level and its level-matching log call into a reusable `RuntimeLevel` type in `ipc_host`
- [x] 2.2 Add a second instance for Lua requests, with `set_lua_request_log_level(LevelFilter)` exported next to `set_key_write_log_level`

## 3. Request handling

- [x] 3.1 In the write path, ahead of the table lookup: `0x0D6C` stores the parameter (up to 4 bytes, little-endian, zero-extended); `0x0D70` logs the request text (up to NUL, max 40 bytes, escaped) with the stored parameter or "no param", marked not supported
- [x] 3.2 Neither offset is forwarded, warned about, or counted as a rejected write
- [x] 3.3 Log each distinct request text and parameter once (repeats at trace); `ResetWarnings` forgets logged requests but keeps the parameter

## 4. Configuration

- [x] 4.1 Parse `[log_levels] key_write` and `lua_request` in `xplane_uipc/src/lib.rs` with the existing level parser; keep reading `[settings] key_write_log_level`; when both are set, use `[log_levels]` and warn that the old key is deprecated
- [x] 4.2 Apply both levels on load and on reload (and reset them to `info` when `config.toml` fails to parse, as the key-write level does today)
- [x] 4.3 Move `key_write_log_level` in `xplane_uipc/config.toml` to `[log_levels]` and add `lua_request`, with comments
- [x] 4.4 Update the README logging section

## 5. Tests

- [x] 5.1 `mapped_view` tests: parameter stored from 1-, 2- and 4-byte writes; request text cut at NUL and at 40 bytes; neither offset forwarded or counted as rejected; parameter persists across `process_mapped_view` calls; reads of both offsets are zero-filled
- [x] 5.2 Config tests: `[log_levels]` values parsed; old key used when the new one is missing; new key wins when both are set
- [x] 5.3 `RuntimeLevel` round-trip test (replaces the existing key-write level test)
- [x] 5.4 Deduplication tests: a repeated request is recorded once, a new parameter or text is recorded again, and `reset_warnings` empties the set but keeps the parameter

## 6. Verification

- [x] 6.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test`, `cargo xtask dist`
- [x] 6.2 Sim check: run the client that wrote `0x0D6C`/`0x0D70` and record the logged request text in `mappings reconciliation/` for the follow-up decision
