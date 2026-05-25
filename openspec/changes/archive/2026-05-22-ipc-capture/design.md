## Context

The `ipc_host` crate receives mapped views from an external FSUIPC client via registered Windows messages. The `wnd_proc` function opens each view and calls `process_mapped_view` to parse records. Currently, a bad sentinel causes `process_mapped_view` to abort immediately, discarding all subsequent records in that view. There is no mechanism to inspect what the client sent.

The capture feature adds:
- Optional capture configuration passed at startup
- Runtime on/off toggling via IPC commands
- Resilient parsing that continues past corruption
- Raw byte capture of error-ridden views to disk
- A CLI tool for analyzing captured files

## Goals / Non-Goals

**Goals:**
- Capture raw bytes of mapped views that contain processing errors
- Survive bad sentinels by scanning forward to the next valid record
- Toggle capture at runtime without restarting the IPC host
- Bound disk usage via configurable file limit
- Provide a standalone tool to inspect captured files

**Non-Goals:**
- Structured/parsed capture format (raw bytes only)
- Network-based capture transport
- Automatic capture on every view (errors only)
- Compression or rotation (guardrail + manual cleanup)

## Decisions

### D1: Static mutable state (like WRITE_CHANNEL)

| Option | Verdict |
|---|---|
| Global `LazyLock<Mutex<Option<CaptureState>>>` | **Adopted** — same pattern as existing `WRITE_CHANNEL`, no new infrastructure |
| Window property (SetPropW/GetPropW) | More ceremony, no benefit since both `wnd_proc` and the message pump run on the same thread |

The `Mutex` is needed for `static` interior mutability, not for synchronization — both `wnd_proc` and the `rx` handler run interleaved on the same thread.

### D2: CaptureConfig struct

| Option | Verdict |
|---|---|
| Flat params `(Option<PathBuf>, Option<usize>)` | Simpler now but doesn't scale |
| `CaptureConfig { path, max }` | **Adopted** — groups related options, extensible if more capture settings are added later |

### D3: Resilient parsing via scan-forward

| Option | Verdict |
|---|---|
| Abort on bad sentinel (current) | Loses all subsequent records |
| Skip current record by nBytes | nBytes may be garbage if sentinel is bad |
| **Scan forward for next sentinel** | **Adopted** — reuses existing `find_next_record` logic that already handles zero-reqID padding gaps |

When a bad sentinel is detected at `cur_ptr`, the parser calls `find_next_record(cur_ptr.add(1), 16)` to scan from the byte after the sentinel. If found, it positions `cur_ptr` at the new record header and continues. If not found within the scan window, it breaks (true end of data).

### D4: process_mapped_view returns error_count: usize

| Option | Verdict |
|---|---|
| Return `bool` | Simple but loses diagnostic value |
| Return `usize` | **Adopted** — callers can log or use as capture trigger; inspect tool also benefits |

### D5: Capture-before processing

| Option | Verdict |
|---|---|
| Capture before processing (copy raw bytes first) | **Adopted** — preserves pristine client data; no contamination from our read-response writes |
| Capture after processing | View has been modified in place by read-response writes; harder to analyze |

The capture writes raw bytes of the mapped view as received, before `process_mapped_view` runs. A `Vec<u8>` copy is made before processing; if errors are detected, the copy is written.

### D6: Spawn-per-write for async I/O

| Option | Verdict |
|---|---|
| Synchronous write on window thread | Blocks message dispatch; adds latency under error storms |
| Dedicated writer thread with channel | Cleaner but more ceremony for a debug feature |
| **`thread::spawn` per write** | **Adopted** — fire-and-forget, keeps window thread unblocked, simple |

### D7: chrono for ISO timestamp formatting

| Option | Verdict |
|---|---|
| Manual date math | ~50 lines of leap-year-aware formatting |
| `chrono` | **Adopted** — one-liner, no bugs, worth the dependency |

### D8: View size via VirtualQuery

The mapped view pointer from `MapViewOfFile(handle, FILE_MAP_WRITE, 0, 0, 0)` maps the entire file mapping. `VirtualQuery` on the pointer returns `MEMORY_BASIC_INFORMATION::RegionSize` which gives the exact byte count to capture.

Dead imports `GetFileSize` / `GetFileSizeEx` in `lib.rs` are cleaned up in this change.

### D9: 1000-file guardrail

When `count >= max`, capture state is set to `enabled = false` and a warning is logged. The next `StartCapture` command is required to resume.

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| **Disk filled by error storm** | Configurable guardrail (default 1000), auto-disables |
| **Same-tick filename collision** | Counter suffix `_1`, `_2`, ... |
| **Large view captures** | Raw bytes could be large; `VirtualQuery` reports the actual region size, no surprise truncation |
| **Window thread blocked briefly** | File I/O is on a spawned thread, so the window thread is not blocked |
| **chrono dep added to cdylib** | chrono is a pure Rust library, no ABI concerns for cdylib |
