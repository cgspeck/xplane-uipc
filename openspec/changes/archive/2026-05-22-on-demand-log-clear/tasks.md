## 1. Shared writer types

- [x] 1.1 Add `SharedFileWriter` and `SharedFileGuard` types in `lib.rs` — `SharedFileWriter` holds `Arc<Mutex<File>>` and impls `MakeWriter`; `SharedFileGuard` clones the `Arc` and impls `io::Write` by locking on each call
- [x] 1.2 Add `LogController` struct holding the `Arc<Mutex<File>>` and the log path
- [x] 1.3 Add `static LOG_CONTROLLER: OnceLock<LogController>` alongside the existing `TRACING_FILTER_HANDLE`

## 2. Plumbing the writer into the subscriber

- [x] 2.1 In `XPluginStart`, wrap the opened `File` in `Arc<Mutex<File>>` and create a `SharedFileWriter` from it
- [x] 2.2 Pass `SharedFileWriter` to `fmt::layer().with_writer()` (instead of bare `Mutex<File>`)
- [x] 2.3 Store the `LogController` in `LOG_CONTROLLER` after subscriber init

## 3. Clear function

- [x] 3.1 Implement `clear_log_file()` — flush the mutex-guarded file, create a new truncated file, swap it under the mutex, write "Log file cleared\n" to the new file

## 4. Menu integration

- [x] 4.1 Add `MENU_CLEAR_LOG` constant to `menu.rs`
- [x] 4.2 Add `MENU_CLEAR_LOG` match arm in `menu_handler` that calls `clear_log_file()`
- [x] 4.3 Add "Clear Trace Log" menu item in `build_menu()`

## 5. Verification

- [x] 5.1 Build compiles with no warnings
- [x] 5.2 Manual: launch X-Plane, verify "Clear Trace Log" menu item appears
- [x] 5.3 Manual: clear the log, verify file is truncated and continues logging
- [x] 5.4 Manual: clear the log under heavy tracing, verify file remains valid
