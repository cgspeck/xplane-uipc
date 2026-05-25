## ADDED Requirements

### Requirement: Table populated before IPC accepts connections
The value table SHALL be fully populated with all mappings (static and dataref-based) before the IPC thread begins accepting client connections. This is achieved by calling a synchronous `populate_table()` method on `PluginState` after `load_mappings_and_init()` and before spawning the IPC thread in `XPluginEnable()`.

#### Scenario: Static mappings available on first IPC request
- **WHEN** a client connects and reads a static-value offset (e.g., `0x3304`) immediately after plugin enable
- **THEN** the value is returned from the table without any "not in table" warning

#### Scenario: Dataref-based mappings with resolved handles available on first request
- **WHEN** a client reads a dataref-based offset whose dataref was successfully resolved during `ResolvedMapping::new()`
- **THEN** the current value is returned from the table

#### Scenario: Unresolved dataref mappings deferred to flight loop
- **WHEN** a mapping's dataref could not be resolved at plugin enable time (returns null handle)
- **THEN** the entry is not populated in the initial table load and will be added during the first `update()` cycle if the dataref becomes available

### Requirement: populate_table uses Table::insert for all entries
The `populate_table()` method SHALL use `Table::insert()` to add entries, ensuring `active` and `writable` vectors are correctly maintained alongside `entries`.

#### Scenario: Active vector contains all populated offsets
- **WHEN** `populate_table()` completes
- **THEN** `table.active` contains the offset of every mapping that was successfully populated

#### Scenario: Writable vector contains all writable offsets
- **WHEN** `populate_table()` completes
- **THEN** `table.writable` contains the offset of every mapping marked as writable

### Requirement: update() rebuilds active/writable vectors each cycle
The `PluginState::update()` method SHALL clear and rebuild `table.active` and `table.writable` vectors during each invocation, using `Table::insert()` for all entries instead of direct array assignment.

#### Scenario: No duplicate entries in active vector after multiple updates
- **WHEN** `update()` is called 100 times with the same set of mappings
- **THEN** `table.active` contains each offset exactly once (no duplicates)

#### Scenario: Writable vector reflects current mapping state
- **WHEN** a mapping is removed between update cycles
- **THEN** the offset is no longer present in `table.writable` after the next `update()`
