## Why

During long debugging sessions, the tracing log file (`uipc.log`) grows unbounded. When it becomes too large to conveniently view in an editor, there is no way to clear it without restarting X-Plane. Adding an on-demand clear mechanism lets the developer reset the log mid-session, keeping it manageable.

## What Changes

- Add a "Clear Trace Log" item to the X-Plane plugin menu
- On invocation, flush the current log file, truncate it to zero, and write a "Log file cleared" marker
- Internally, swap the file handle under the tracing writer using `Arc<Mutex<File>>` with a custom `MakeWriter` implementation
- The existing `Mutex<File>` pattern in `with_writer()` is replaced with a rotatable wrapper

## Capabilities

### New Capabilities

- `on-demand-log-clear`: The ability to clear the tracing log file at runtime via a plugin menu command, without restarting X-Plane or losing subsequent trace output.

### Modified Capabilities

_(None — no existing spec-level behavior changes)_

## Impact

- **`xplane_uipc/src/lib.rs`**: Add `SharedFileWriter` / `SharedFileGuard` types, a `LOG_CONTROLLER` static, and a `clear_log_file()` public function. Replace the `Mutex<File>` passed to `with_writer()` with the new wrapper.
- **`xplane_uipc/src/menu.rs`**: Add `MENU_CLEAR_LOG` constant, match arm, and menu item.
- **No new dependencies.**
