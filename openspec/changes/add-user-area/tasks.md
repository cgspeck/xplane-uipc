## 1. IPC window state

- [ ] 1.1 If `log-lua-requests` hasn't landed yet: add `IpcState { warned: WarnedSet, … }` in `ipc_host`, store it in `GWLP_USERDATA` instead of the bare `WarnedSet`, and update `create_ipc_window`, `wnd_proc`, the `ResetWarnings` handler (clears `warned` only), teardown and `process_mapped_view` (takes `&mut IpcState`)
- [ ] 1.2 Add `user_area: [u8; 64]`, zeroed on creation, and export `USER_AREA: RangeInclusive<u16> = 0x66C0..=0x66FF` from `ipc_host`

## 2. Read and write paths

- [ ] 2.1 In `answer_read`, before the table lookup: for `dwOffset` in `USER_AREA`, copy the stored bytes up to the end of the area and zero-fill the rest of the payload
- [ ] 2.2 In `apply_write`, before the `0x8001` and table handling: for `dwOffset` in `USER_AREA`, store the bytes that fall inside the area, log the write at debug (offset, size, hex bytes), and log at debug if bytes were dropped past the end; never forward or count as rejected

## 3. Mappings

- [ ] 3.1 Drop mappings whose offset is in `USER_AREA` at load, with a warning, next to `drop_builtin_offsets` in `xplane_uipc/src/plugin_state.rs`

## 4. Tests

- [ ] 4.1 `mapped_view` tests: write then read back in a later `process_mapped_view` call; unwritten bytes read as zero with no warning; write clipped at `0x66FF`; read past the end zero-filled; a write in one call is visible to a read in the same view; nothing forwarded and no rejected writes
- [ ] 4.2 `plugin_state` test: a mapping inside the user area is dropped and others are kept

## 5. Docs

- [ ] 5.1 README: describe the user area next to the built-in `0x337E` offset

## 6. Verification

- [ ] 6.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test`, `cargo xtask dist`
- [ ] 6.2 Sim check with FSInterrogate: write 8 bytes to `0x66F8` (as a raw/hex field), read them back, then reload mappings and read again; confirm no "not in table" warning for `0x66F8` in `uipc.log`
