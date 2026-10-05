## Why

The `ipc_host` parser was reverse-engineered from SLC captures and doesn't match the FSUIPC wire protocol defined by the SDK client libraries (`IPCuser.c` in `UIPC_SDK_C.zip`, `FSUIPCuser64.c` in `UIPC64_SDK_C_version2.zip`). It assumes every record has a 16-byte header whose 4th field is a `"luaP"` sentinel, and that bit 31 of `nBytes` marks a write. In the SDK:

- The first DWORD (`dwId`) gives the record type: `1` = read (32-bit client), `4` = read (64-bit client), `2` = write, `0` = end of stream.
- A read header holds the client's own destination pointer `pDest` (4 bytes for 32-bit clients, 8 bytes packed for 64-bit). `"luaP"` is just the .NET client library putting `"Paul"` in that slot.
- A write header is 12 bytes and has no 4th field. No size flag exists.

The result is that every write record is misparsed: the parser takes the first 4 payload bytes as a "sentinel" and loses alignment for the rest of the view. FSInterrogate's connect sequence shows this (`unhandled-req-issue/`). Its 13-byte key write to `0x8001` is read as a read, and the next "record" is junk (`dwOffset=0xff010aff`). Writes from SLC and other clients have probably never been decoded correctly. 64-bit SDK clients (`dwId=4`, 20-byte header) aren't supported at all.

Reads of offsets we don't serve also leave the client's payload untouched. FSUIPC itself always returns bytes (zeros for unused offsets). So a client polling something like `0x337E` before it is populated reads back whatever stale bytes were in its buffer.

## What Changes

- **BREAKING (internal API)**: Rewrite `iterate_records` to dispatch on `dwId`: read32 (16-byte header), read64 (20-byte header), write (12-byte header), terminator (`0`). An unknown `dwId` or a record that overruns the view stops the scan and counts as an error.
- Remove the bit-31 write flag, the `"luaP"` sentinel check, `find_next_record` padding/recovery scanning and the `:FSD` diagnostics. They only existed because of the misparse.
- `ParsedRecord` reports a record kind (`Read32`, `Read64`, `Write`), header offset, `dwOffset`, `nBytes`, payload pointer and, for reads, `pDest` (diagnostics only).
- Reads of offsets not in the table zero-fill the requested payload bytes, as FSUIPC does.
- Writes to `0x8001` (application key registration, used by FSInterrogate) are accepted and logged with the key text. They aren't treated as writes to an unmapped offset. The log level comes from a new `key_write_log_level` setting in `config.toml` (default `info`, `off` silences it).
- The window procedure returns `FS6IPC_MESSAGE_SUCCESS` (1) when the view parses cleanly and `FS6IPC_MESSAGE_FAILURE` (0) when it is malformed, so SDK clients report `FSUIPC_ERR_DATA` instead of copying garbage.
- Update `capture-inspect` output, the test fixtures and tests, and the protocol section of `CLAUDE.md`.

## Capabilities

### New Capabilities
- `fsuipc-ipc-protocol`: How `ipc_host` parses and answers the FSUIPC shared-memory request stream: record framing per `dwId`, read/write semantics, unknown offsets, the `0x8001` key write, and the message result.

### Modified Capabilities
- `capture-inspect`: Record display follows the new record kinds. Sentinel markers and gap display are removed.
- `error-capture`: The "bad sentinel" triggers and resilient-parsing requirements are replaced by "malformed view" semantics (stop the scan, count an error).

## Impact

- **`ipc_host/src/mapped_view.rs`**: Parser rewrite. `process_mapped_view` gains zero-fill and `0x8001` handling.
- **`ipc_host/src/lib.rs`**: `wnd_proc` returns 0 on malformed views. New setter for the key-write log level.
- **`xplane_uipc/src/lib.rs`**: Read `key_write_log_level` from `config.toml` and pass it to `ipc_host`.
- **`ipc_host/examples/capture-inspect.rs`**, **`ipc_host/tests/capture_records.rs`**: Updated for the new `ParsedRecord`. New fixtures from `unhandled-req-issue/`.
- **`config.toml`** (repo and `dist`), **README**, **`CLAUDE.md`**: Document the setting and the corrected protocol.
- **Clients**: SLC reads are unaffected (same 16-byte read framing). SLC/SPAD writes start being decoded correctly. 64-bit clients become supported.
