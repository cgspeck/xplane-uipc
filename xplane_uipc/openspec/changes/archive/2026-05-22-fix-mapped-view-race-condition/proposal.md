## Why

IPC clients receive "Read from offset 0xXXXX not in table" warnings for offsets that have valid mappings (e.g., `0x3304`, `0x3308`). This happens because the IPC thread begins processing client requests before the flight loop's `PluginState::update()` has populated the value table. The table starts empty, and entries are only added during `update()` which runs at 20Hz — the first IPC request can arrive before the first `update()` call. Additionally, the `WarnedSet` permanently records "not found" warnings, so the initial failure is logged even after the table is later populated.

## What Changes

- The value table will be fully populated synchronously before the IPC thread begins accepting client connections
- The `WarnedSet` will no longer permanently suppress warnings for offsets that later become valid
- The `Table::insert()` method will be used for all entry additions so that `active` and `writable` vectors stay in sync with `entries`

## Capabilities

### New Capabilities
- `table-readiness`: The value table must be fully populated with all static and dataref-based mappings before the IPC server accepts client connections. Read requests for valid mappings must never return "not in table" due to initialization ordering.
- `warning-lifecycle`: The warning deduplication system (`WarnedSet`) must allow offsets to transition from "not found" to "found" without permanently suppressing future state. Warnings should reflect the current state of the table, not just the first observation.

### Modified Capabilities
- None

## Impact

- `xplane_uipc/src/lib.rs`: IPC thread startup sequence — table population must happen before `IpcHost::run()`
- `xplane_uipc/src/plugin_state.rs`: `update()` method — should use `Table::insert()` instead of direct array assignment
- `ipc_host/src/mapped_view.rs`: Read/write path — may need adjustment if WarnedSet behavior changes
- `ipc_host/src/warning.rs`: `WarnedSet` — may need a reset or transition mechanism
- `ipc_host/src/value_table.rs`: `Table::insert()` becomes the canonical way to add entries
