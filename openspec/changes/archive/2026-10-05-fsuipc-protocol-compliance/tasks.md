## 1. Parser

- [x] 1.1 In `ipc_host/src/mapped_view.rs`, add constants `FS6IPC_READSTATEDATA_ID = 1`, `F64IPC_READSTATEDATA_ID = 4`, `FS6IPC_WRITESTATEDATA_ID = 2`, `APP_KEY_OFFSET = 0x8001`, and a `RecordKind { Read32, Read64, Write }` enum with header sizes 16/20/12
- [x] 1.2 Replace `ParsedRecord` with `{ kind, header_offset, dw_offset, n_bytes, p_dest: u64, payload_ptr }`
- [x] 1.3 Rewrite `iterate_records`: bounds-check each header and payload against `view_size`, dispatch on `dwId`, stop at `0`, and stop with one error (warn with byte offset, reason and the fields read) on an unknown id, an overrun or a missing terminator
- [x] 1.4 Delete `SENTINEL`, `FSD_SENTINEL`, `find_next_record` and the `:FSD` diagnostics

## 2. Request handling

- [x] 2.1 In `process_mapped_view`, zero-fill the payload for reads of offsets not in the table, or above `0xFFFF`
- [x] 2.2 Skip writes with `dwOffset > 0xFFFF` (debug log, no error), and stop truncating `dw_offset as u16` without checking it first
- [x] 2.3 Handle writes to `APP_KEY_OFFSET`: extract printable text up to NUL/`nBytes`, log it at the configured level, no forwarding and no warnings
- [x] 2.4 Add `set_key_write_log_level(LevelFilter)` / a static `AtomicU8` in `ipc_host`, plus a helper that emits at a runtime-chosen level
- [x] 2.5 In `wnd_proc` (`ipc_host/src/lib.rs`), return `LRESULT(0)` when `process_mapped_view` reports a malformed view (rejected writes alone don't fail the message) and `LRESULT(1)` otherwise; rename the magic numbers to `FS6IPC_MESSAGE_SUCCESS`/`FAILURE`

## 3. Configuration

- [x] 3.1 Add `key_write_log_level: Option<String>` to `Settings` in `xplane_uipc/src/lib.rs`; parse it in `parse_config_and_apply` (missing → info, invalid → warn + info) and call the `ipc_host` setter
- [x] 3.2 Add `key_write_log_level = "info"` to the repo `config.toml` and the shipped/dist config, with a comment

## 4. Tooling

- [x] 4.1 Update `ipc_host/examples/capture-inspect.rs` to the new display format (kind, header offset, pDest or write bytes, MALFORMED line)
- [x] 4.2 Check `uipc-debug` and any other users of `ParsedRecord`/`is_write` still build

## 5. Tests

- [x] 5.1 Replace the `"luaP"`-based test helpers in `mapped_view.rs` with SDK-shaped builders (`read32`, `read64`, `write`, `end`) and port the existing read/write/narrow-read/string tests to them
- [x] 5.2 Parser tests: Read64 framing; mixed read/write/read stream; unknown dwId stops with 1 error; payload overrun stops with 1 error; missing terminator counts an error; `nBytes == 0` is accepted
- [x] 5.3 Handling tests: an unknown offset read is zero-filled; a `0x8001` write isn't forwarded and doesn't count as an error; a write to a writable offset via `dwId=2` is forwarded with the decoded value
- [x] 5.4 `ipc_host/tests/capture_records.rs`: keep the SLC and FSInterrogate read fixtures; turn `test_8001_dump_does_not_crash` into a positive assertion (one Write, `0x8001`, 13 bytes, payload at `0x0C`, 0 errors); add the `unhandled-req-issue` `0x337E` read capture as a fixture
- [x] 5.5 Config parsing test for `key_write_log_level` (valid, missing, invalid), if `parse_config_and_apply` can be split into a testable pure function

## 6. Docs and verification

- [x] 6.1 Rewrite the "FSUIPC binary protocol" section of `CLAUDE.md` (record table, no sentinel, result codes, SDK source references)
- [x] 6.2 Document `key_write_log_level` in the README config section
- [x] 6.3 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test` and `cargo xtask dist`
- [x] 6.4 Manual check: deploy, connect FSInterrogate with `log_level = "debug"`, confirm no malformed-view warnings and see whether the status still alternates; capture an SLC write session for a future fixture (FSInterrogate passed: key write, 0x337E polls, stable connection. No SLC write captured yet; an FSInterrogate write to 0x023B was captured instead.)
