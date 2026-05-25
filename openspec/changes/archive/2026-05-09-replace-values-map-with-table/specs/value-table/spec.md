## ADDED Requirements

### Requirement: Value table is initialized at plugin startup
The system SHALL initialize the value table structure during XPluginEnable, populating it with the required FSUIPC offset mappings before any messages are processed.

#### Scenario: Table initializes with expected offsets
- **WHEN** XPluginEnable is called by X-Plane
- **THEN** the table is created with entries for offsets 0x3304, 0x3308, 0x3124, and 0x320c populated with their default values

### Requirement: Value table provides O(1) lookups
The system SHALL provide constant-time lookups for any u16 offset value by accessing the table entries array directly.

#### Scenario: Lookup returns entry for known offset
- **WHEN** uipc_host requests the entry for offset 0x3304
- **THEN** the table returns the corresponding Entry with Value::Integer and source/destination values in O(1) time

#### Scenario: Lookup returns None for unknown offset
- **WHEN** uipc_host requests the entry for an offset not in the table
- **THEN** the table returns None

### Requirement: Value table is thread-safe
The system SHALL allow concurrent read access from uipc_host while writes occur only during initialization in lib.rs.

#### Scenario: Concurrent read during message processing
- **WHEN** uipc_host reads from the table while no writes are occurring
- **THEN** the read succeeds without blocking

### Requirement: Entries can be cloned for write operations
The system SHALL allow Entry values to be cloned so they can be written into the shared memory view without borrowing the table.

#### Scenario: Value is cloned for memory write
- **WHEN** uipc_host retrieves an Entry from the table
- **THEN** it can clone the Value and write it to the mapped memory view