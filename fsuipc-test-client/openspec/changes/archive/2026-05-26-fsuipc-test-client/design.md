## Context

The `fsuipc-test-client` is a new .NET console app that reads FSUIPC offsets and displays their values. It lives alongside the existing Rust `uipc-debug` tool and `ipc_host` crate. Where `uipc-debug` evaluates mapping expressions from X-Plane datarefs and writes values to the shared value table, this tool approaches from the other direction: it connects as an FSUIPC client (via `FSUIPCClientDLL`) to whatever FSUIPC-compatible host is running (`UIPCMAIN` window) and reads raw offsets directly.

## Goals / Non-Goals

**Goals:**
- Parse a text file listing offset addresses with types/sizes
- TUI: live table of offset values refreshing at ~1-2 Hz
- TUI: reload input file (`r`), save snapshot to JSON (`s`), navigate rows (`↑/↓`)
- Batch mode: one-shot query, JSON to stdout
- Mirror `uipc-debug` keybindings where applicable
- Zero configuration — auto-connect to any FSUIPC host

**Non-Goals:**
- Not a mapping expression evaluator (that's `uipc-debug`'s role)
- No write support in initial version (read-only)
- No WideClient / multi-instance support initially
- No CSV output (JSON only via `s` key or batch mode)

## Decisions

**1. Input file format: `<offset>,<type>[,<size>]`**

Fixed-size types omit `size`:
| Type | Size | Example |
|------|------|---------|
| `u8`, `i8` | 1 | `0x3365,u8` |
| `u16`, `i16` | 2 | `0x0D0C,u16` |
| `u32`, `i32`, `f32` | 4 | `0x02BC,i32` |
| `u64`, `i64`, `f64` | 8 | `0x0560,i64` |
| `string` | required | `0x3160,string,24` |
| `bytes` | required | `0x0238,bytes,10` |

Comments with `#` and blank lines are ignored.

**2. Spectre.Console for TUI**

It provides `AnsiConsole.Write()` for tables, `LiveDisplay` for auto-refresh, and `ConsoleKeyInput` for keyboard handling. No need for raw terminal mode or P/Invoke.

**3. Offset management**

On file load / reload:
1. Parse each line → `(address, type, size)` tuple
2. Create corresponding `Offset<T>` objects (typed for known sizes, `Offset<byte[]>` for raw)
3. Call `FSUIPCConnection.Process()` to batch-read all registered offsets
4. On reload (`r`), discard old offsets (they disconnect on GC), parse and create new ones

**4. TUI architecture**

```
Event Loop (every ~500ms)
  ├─ Draw table (Spectre.Console LiveDisplay)
  ├─ Poll keyboard
  │   ├─ ↑/↓ → adjust selected row
  │   ├─ r → reparse file, reconnect offsets
  │   ├─ s → serialize current values to JSON, write to file
  │   ├─ q → quit (close connection, exit)
  │   └─ ? → show help overlay
  └─ FSUIPCConnection.Process() → update all offset values
```

**5. JSON output format**

```json
{
  "timestamp": "2026-05-26T12:00:00.000Z",
  "source": "uipc-debug",
  "offsets": [
    { "address": "0x02BC", "type": "i32", "value": 250 },
    { "address": "0x3160", "type": "string", "size": 24, "value": "B737" }
  ]
}
```

**6. Auto-connect (no targeting)**

The FSUIPCClientDLL's `FSUIPCConnection.Open()` uses `FindWindowEx(0, 0, "UIPCMAIN", Nil)` — the same window class that `ipc_host` registers and that FSUIPC7 registers. Whichever host is running gets connected. No `--target` flag needed.

## Risks / Trade-offs

- **[Compatibility]** If the FSUIPCClientDLL version bumps change the `Offset<T>` API, updates may be needed. → Pin to a known-good version in the `.csproj`.
- **[Both hosts running]** If both MSFS+FSUIPC7 and uipc-debug are running, connection goes to whichever `UIPCMAIN` window `FindWindowEx` finds first. → Unlikely in practice; document the behavior.
- **[Flicker in TUI]** Spectre.Console `LiveDisplay` redraws the full table each cycle. → Acceptable at 1-2 Hz with small offset lists. Large lists (~1000+) may need pagination.
