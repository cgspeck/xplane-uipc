## Context

Currently, `uipc_host.rs` contains a hard-coded `HashMap<u32, ValueType>` inside `wnd_proc()` that maps FSUIPC offset values (like `0x3304`, `0x3308`) to their types and default values. This HashMap is created fresh on every Windows message, making it inefficient and coupling the value definitions to the message handling code.

The X-Plane plugin calls `XPluginEnable` in `lib.rs` during startup, which is the appropriate place to initialize plugin-wide state.

## Goals / Non-Goals

**Goals:**
- Replace hard-coded HashMap with a table structure defined once at plugin startup
- Use `Arc<RwLock<Table>>` for thread-safe sharing between lib.rs (write) and uipc_host.rs (read)
- Initialize table in `XPluginEnable` in lib.rs
- Maintain all existing offset-to-value mappings

**Non-Goals:**
- Adding new FSUIPC offsets (this is just a refactoring)
- Persisting values to disk
- Dynamic runtime modification of the table

## Decisions

1. **Table Structure over HashMap**: Use `Box<[Option<Entry>; 65536]>` instead of HashMap for O(1) lookups without hashing overhead.

   - Alternative considered: Keep HashMap and wrap in Arc<RwLock> - rejected because it adds unnecessary hash computation per message and recreates the map for each window message.
   - Rationale: 65536 entries (one per possible u16 offset) provides direct array access with minimal indirection.

2. **RwLock for Synchronization**: Use `std::sync::RwLock` instead of `parking_lot::RwLock` or `tokio::sync::RwLock`.

   - Rationale: X-Plane plugins run on the main thread; tokio is not used. parking_lot adds an extra dependency. std::sync::RwLock is sufficient.

3. **Value Enum with Clone**: Derive `Clone` on `Value` enum to allow copying entries.

   - Rationale: Read operations need to clone the value to write into the shared memory view.

4. **Active Vector for Iteration**: Maintain `active: Vec<u16>` to track which indices have entries.

   - Rationale: The Box array is sparse; iterating over all 65536 entries would be inefficient. The active vector allows efficient iteration over only populated entries if needed.

## Risks / Trade-offs

- **[Risk]** Larger memory footprint: The Box array uses ~65536 * size_of::<Option<Entry>> (~16 bytes) ≈ 1MB.
  
  → **Mitigation**: Acceptable for a plugin. The HashMap approach also kept entries in memory. Can be reduced to `Vec<Option<Entry>>` if memory is constrained.

- **[Risk]** Runtime initialization in XPluginEnable could block plugin startup.
  
  → **Mitigation**: The table initialization is trivial (just setting Option::None and populating a handful of entries). No concern in practice.