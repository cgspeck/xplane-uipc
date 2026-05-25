## Context

The tracing subscriber in `xplane_uipc` writes to `uipc.log` via `fmt::layer().with_writer(Mutex<File>)`. The `Mutex<File>` is consumed at init — nothing outside the layer can reach the file handle to truncate or swap it. The only mechanism to clear the log today is restarting X-Plane (which re-triggers `XPluginStart` and reopens with `truncate(true)`).

## Goals / Non-Goals

**Goals:**
- Add a "Clear Trace Log" menu item that truncates the log file mid-session
- All subsequent trace output continues writing to the cleared file
- Zero data corruption risk under concurrent tracing

**Non-Goals:**
- Log archival or rotation (user wants simple clear)
- Automatic size-based truncation
- IPC-triggered clearing
- Per-module log filtering (already solved by configurable tracing level)

## Decisions

### 1. Custom `MakeWriter` wrapping `Arc<Mutex<File>>` instead of bare `Mutex<File>`

- **Chosen**: Create `SharedFileWriter` (implements `MakeWriter`) and `SharedFileGuard` (implements `io::Write`). The writer holds an `Arc<Mutex<File>>`; the guard clones the `Arc` and locks on each `write()` call.
- **Alternative considered**: OS-level `SetEndOfFile` truncation on the existing `File`. Rejected because there's no way to reach the locked file handle from outside the layer without changing the writer architecture.
- **Alternative considered**: `tracing-appender` with `RollingFileAppender`. Rejected because it doesn't support on-demand rotation.
- **Alternative considered**: Re-initializing the subscriber. Rejected because `tracing` does not support re-init of a global subscriber.

### 2. Per-call locking in the guard, not per-event

- Each `write()` call locks the mutex independently, rather than holding the lock across all writes for a single event.
- **Tradeoff**: A single formatted event (timestamp, level, target, message — typically 4-5 write calls) could be split across a rotation boundary. In practice: extremely narrow window, and the effect is at most one interleaved line. Acceptable for a debug tool.
- **Counter-argument**: Holding the lock across the full format reduces risk but adds contention to every trace event. Not worth it for this use case.

### 3. `LOG_CONTROLLER` in a `OnceLock` static

- A `OnceLock<LogController>` stores the `Arc<Mutex<File>>` and the log path string. Set during `XPluginStart`, read by the menu handler.
- **Alternative considered**: Passing the controller through the menu handler's `refcon`. Works but adds indirection. `OnceLock` is simpler and matches the existing `TRACING_FILTER_HANDLE` pattern.

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| One trace event splits across old/new file during rotation | Acceptable for debug tool. At worst one line is fragmented. |
| Forgetting to flush before swap loses buffered data | Flush the mutex-guarded file before swapping. |
| Menu handler deadlocks if tracing writes hold the mutex | Rotation acquires the lock briefly; tracing writes do the same. No nested locking. |
| Old file handle leaks RAM (never dropped) | `Arc` drops old `File` when last guard referencing it finishes. |
