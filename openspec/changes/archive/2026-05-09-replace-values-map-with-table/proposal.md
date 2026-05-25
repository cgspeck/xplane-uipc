## Why

The current implementation uses a hard-coded `HashMap` inside `uipc_host.rs` `wnd_proc()` to map FSUIPC offset values to their types and default values. This makes it difficult to modify the value mappings at runtime and couples the data tightly to the message handling logic. A table-based structure initialized in `lib.rs` during `XPluginEnable` would allow the mappings to be configured once and shared across the plugin lifetime.

## What Changes

- Replace the hard-coded `values_map: HashMap<u32, ValueType>` in `uipc_host.rs` with a table structure defined in `lib.rs`
- Create a new `Table` struct with `entries: Box<[Option<Entry>; 65536]>` and `active: Vec<u16>` to store all possible offset mappings
- Wrap the table in `Arc<RwLock<Table>>` for safe concurrent access between `lib.rs` (write) and `uipc_host.rs` (read)
- Define `Value` enum and `Entry` struct as shared types
- Move value initialization to `XPluginEnable` in `lib.rs`

## Capabilities

### New Capabilities

- `value-table`: A table-based structure for storing FSUIPC offset value mappings, initialized at plugin startup and read during message processing

### Modified Capabilities

- (none - this is a pure refactoring with no specification-level behavior changes)

## Impact

- `xplane-uipc/src/uipc_host.rs`: Replace HashMap lookups with table lookups
- `xplane-uipc/src/lib.rs`: Initialize table during `XPluginEnable`
- New types (`Table`, `Entry`, `Value`) added to shared module