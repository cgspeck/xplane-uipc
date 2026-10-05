## Context

Reference: the FSUIPC SDK client libraries in `FSUIPC SDK/`:

- `UIPC_SDK_C.zip` → `Lib_source.zip` → `IPCuser.h` / `IPCuser.c` (32-bit)
- `UIPC64_SDK_C_version2.zip` → `Lib64_source.zip` → `FSUIPCuser64.h` / `FSUIPCuser64.c` (64-bit)

```c
#define FS6IPC_MESSAGE_SUCCESS 1
#define FS6IPC_MESSAGE_FAILURE 0
#define FS6IPC_READSTATEDATA_ID    1   // 32-bit lib
#define F64IPC_READSTATEDATA_ID    4   // 64-bit lib
#define FS6IPC_WRITESTATEDATA_ID   2   // both

// read:  DWORD dwId; DWORD dwOffset; DWORD nBytes; void* pDest;   (64-bit: #pragma pack 1)
// write: DWORD dwId; DWORD dwOffset; DWORD nBytes;
```

The client builds records back to back in its file mapping (header, then `nBytes` of payload), ends the stream with a zero `dwId`, and sends the registered `FsasmLib:IPC` message with the mapping's atom. After `SendMessageTimeout`, a result other than `FS6IPC_MESSAGE_SUCCESS` gives `FSUIPC_ERR_DATA`. Otherwise the client walks the same records and copies each read's payload to `pDest`.

Wire layouts:

| dwId | Kind | Header | Payload at |
|---|---|---|---|
| 1 | Read32 | `[id:4][off:4][n:4][pDest:4]` | +16 |
| 4 | Read64 | `[id:4][off:4][n:4][pDest:8]` | +20 |
| 2 | Write | `[id:4][off:4][n:4]` | +12 |
| 0 | End | — | — |

Example of FSInterrogate's key write (`unhandled-req-issue/…30.097Z.bin`):

```
00: 02000000 01800000 0d000000   Write, 0x8001, 13 bytes
0C: "6PETEXPDRVW3\0"
19: 00000000                     End
```

## Goals / Non-Goals

**Goals:**
- Parse exactly what the SDK clients emit, for 32- and 64-bit clients.
- Fail closed on malformed data: stop, count an error, tell the client.
- Answer reads the way FSUIPC does, including offsets we don't serve.
- Accept the `0x8001` key write quietly, with a configurable log level.

**Non-Goals:**
- Offsets above `0xFFFF`. The FSUIPC offset space is 64 KB: "FSUIPC for Programmers" documents nothing past `0xFFFF`, and its highest area is the AI traffic block at `D000`–`FFFF`. The `dwOffset > 0xFFFF` values seen so far were artefacts of the misparse. A well-framed record with such an offset is still handled defensively: reads are zero-filled, writes are ignored, and it's logged once. It's not treated as malformed.
- Validating or enforcing the application key. FSUIPC registration isn't emulated.
- Writing to `pDest`. It's a pointer in the client's address space.
- Fixing FSInterrogate's "responding / not responding" alternation directly. This change removes two likely contributors (the misparse and stale bytes on unknown reads). Whatever is left gets investigated afterwards with debug logs.

## Decisions

1. **Dispatch on `dwId`, with header size by kind.** A `RecordKind` enum (`Read32`, `Read64`, `Write`) replaces `is_write`/`sentinel_ok`. `ParsedRecord` becomes `{ kind, header_offset, dw_offset, n_bytes, p_dest: u64 (0 for writes), payload_ptr }`. Before reading each header and payload, the parser checks bounds against `view_size`.

2. **Malformed means stop.** These end the scan and add one error:
   - an unknown `dwId`
   - a header or payload past the end of the view
   - running out of view without a terminator

   No resync scanning is done. The SDK client also stops at an unknown id (it zeroes it), so nothing after it is meaningful. `nBytes == 0` is legal and just has no payload; the SDK only skips the copy.
   - *Alternative:* keep the `find_next_record` heuristics. Rejected: once a record is misframed, nothing marks where the next one starts, and the heuristics are what produced the junk-record warnings.

3. **Return `FS6IPC_MESSAGE_FAILURE` for malformed views.** `wnd_proc` currently always returns 1. Returning 0 on a malformed view makes the SDK client report `FSUIPC_ERR_DATA` and skip copying reads, instead of consuming partly-answered data. Reads and writes before the bad record have already been applied, as with a partial FSUIPC run. Early-exit paths in `wnd_proc` (atom/mapping/view failures) keep their current return values. Changing those is out of scope.
   - *Risk:* a client that disconnects on any error might drop on one bad view. Acceptable, because a well-formed client shouldn't produce one any more.

4. **Zero-fill unknown reads.** When an offset isn't in the table, the payload's `nBytes` are set to 0. String reads already pad with zeros. The not-in-table warning stays as is (once per offset, debug per read).

5. **`0x8001` key write handling.** In `process_mapped_view`, a write to `0x8001` (constant `APP_KEY_OFFSET`) is handled before the table lookup. It logs `Application key write: "<text>"` (printable ASCII up to the first NUL, at most `nBytes`) at the configured level, and is never forwarded or warned about as non-active.
   - The level is held in `ipc_host` as a static `AtomicU8` encoding a `tracing::level_filters::LevelFilter` (OFF/ERROR/WARN/INFO/DEBUG/TRACE), set with `ipc_host::set_key_write_log_level(LevelFilter)`. The plugin calls it from `parse_config_and_apply`, so it follows "Reload Mappings" like `log_level`. A static is used rather than an `IpcCommands` variant because it's a single value read on the IPC thread with no ordering requirements.
   - Config: `[settings] key_write_log_level = "info"`. Missing → `info`. Invalid → warn and use `info`. The global `log_level` filter still applies on top, so `key_write_log_level = "debug"` only shows up when `log_level` is `debug` or finer.
   - *Alternative:* emit at a fixed level under a dedicated tracing target, filtered by `EnvFilter`. Rejected: the plugin uses a single `LevelFilter` reload handle, and adding per-target filtering is a bigger change than one setting.

6. **Remove sentinel-era code.** Delete `SENTINEL`, `FSD_SENTINEL`, `find_next_record` and the `:FSD` text detection, plus `raw_n`, `sentinel_ok`, `sentinel_offset` and `recovery_next_offset` from `ParsedRecord`. Tests built on `"luaP"` framing are rewritten to build SDK-shaped records with a small helper (`read32(off, n)`, `read64(off, n)`, `write(off, bytes)`, `end()`).

7. **Fixtures.** Keep `slc-3-reads.bin` and `fsinterrogate-2-reads.bin`. They are valid Read32 streams and must still parse to the same offsets and sizes. `fsinterrogate-offset-8001.bin` becomes a positive test: one `Write` to `0x8001` with 13 bytes, then End, with 0 errors. Add a synthetic Read64 fixture built in-test, since we have no 64-bit capture.

## Risks / Trade-offs

- [Medium] **The SLC write format is unconfirmed.** We haven't captured an SLC write. The .NET client talks to real FSUIPC, which only understands `dwId=2`, so SLC writes should follow the SDK. The bit-31 path had no evidence behind it. *Mitigation:* capture an SLC write session after deploying and add it as a fixture.
- [Low] **Returning 0 changes client-visible behaviour.** Before, malformed views got 1. See decision 3.
- [Low] **Zero-fill overwrites client buffer contents** for unknown offsets. This matches FSUIPC, and clients shouldn't rely on stale bytes.

## Open Questions

- After deploying, does FSInterrogate still alternate between "responding" and "not responding"? Run with `log_level = "debug"` to see every `0x337E` read.
