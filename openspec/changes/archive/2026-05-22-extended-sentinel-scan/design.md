## Context

`iterate_records` currently has no knowledge of the mapped view's total size. It relies
on zero-reqID as a terminator and uses a 16-byte `find_next_record` window for both
padding-skip and bad-sentinel recovery. When a bad sentinel is encountered and the
16-byte scan fails, the loop breaks immediately — even if valid records exist further
on.

The `:FSD` pattern is a real observed case where ~130 bytes of ASCII + zeros sit between
orphan 12-byte records and properly-formed frames.

## Design

### D1: Pass `view_size` into `iterate_records`

`iterate_records` gains a `view_size: usize` parameter. The caller is responsible for
providing the actual byte count of the mapped view. This enables safe bounded scanning
of the entire remaining buffer.

| Caller | Source of `view_size` |
|--------|-----------------------|
| `process_mapped_view` | Threaded from `wnd_proc` (which gets it from `VirtualQuery`) |
| `capture-inspect` | `data.len()` on the read file buffer |

### D2: Two-phase sentinel recovery

When a bad sentinel is detected at `sentinel_offset`:

1. **Phase 1** (same as today): `find_next_record(cur_ptr + 1, 16)` — scan 16 bytes
2. **Phase 2** (new): If Phase 1 fails, call `find_next_record` with the remaining
   view size: `max_gap = view_size.saturating_sub(sentinel_offset + 1 + 12)`

If Phase 2 fails too, the loop breaks (true end of data / no "luaP" anywhere).

```
  sentinel_offset            view_size
  │                           │
  ▼                           ▼
  ┌──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┐
  │  │FS│  │  │  │  │  │  │  │lu│aP│  │  │
  └──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┘
  ▲     ▲──────────────────▲
  │     │   Phase 1 (16)  │
  bad   └───── fails ─────┘
                              ▲──────────▲
                               Phase 2 (rest)
```

### D3: Once-logging of bad sentinel values

Maintain a `std::sync::LazyLock<Mutex<HashSet<u32>>>` to track which distinct
bad sentinel values have been logged. For each bad sentinel encountered, the
handler locks the set; if the value is newly inserted (value not previously
seen), it logs the hex value at `warn` level.

A separate `AtomicBool` flag guards the `:FSD` trailing-text log — that text
only needs to be sampled once per process lifetime regardless of how many
different `:FSD` variants appear.

### D4: `:FSD` text extraction

On `:FSD` detection, extract trailing text up to 255 bytes:

```
skip non-printable / non-ASCII bytes at start (after the 4-byte sentinel)
then read printable ASCII until null byte or 255-byte limit
```

If any text was extracted, log once at `info` level:
```
"Bad sentinel: bytes at {:#x} are ':FSD' followed by text: \"{}\""
```

## Updated resilient parsing flow

```
loop:
  read reqID
  if reqID == 0: handle padding / terminator
  read dwOffset
  read nBytes
  read sentinel at offset S

  if sentinel != "luaP":
    error_count += 1
    log_sentinel_value_once(S, sentinel_value)
    if sentinel_value == ":FSD":
      log_fsd_text_once(S, trailing_bytes)

    match find_next_record(S+1, 16):
      Some(n) → advance by n, continue
      None     → match find_next_record(S+1, view_size - S - 1 - 12):
                   Some(n) → advance by n, continue
                   None     → break (end of data)

  else:
    read payload at cur_ptr
    process record
    advance past payload
```

## File changes

| File | Change |
|------|--------|
| `ipc_host/src/mapped_view.rs` | Add `view_size` param to `iterate_records`; two-phase scan; once-logging; `:FSD` text extraction |
| `ipc_host/src/lib.rs` | Pass `view_size` from `VirtualQuery` through to `process_mapped_view` / `iterate_records` |
| `ipc_host/examples/capture-inspect.rs` | Pass `data.len()` as `view_size` to `iterate_records` |
| `ipc_host/src/mapped_view.rs` (tests) | Add tests for `:FSD` junk, extended recovery, once-logging, mixed valid/junk buffers |
