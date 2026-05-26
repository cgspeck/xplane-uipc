## ADDED Requirements

### Requirement: --no-ipc CLI flag
The tool SHALL accept an optional `--no-ipc` CLI flag. When absent (default), the tool SHALL start the IPC host thread and populate the IPC value table from evaluated mappings. When provided, the tool SHALL run in offline mode with no IPC host, preserving all existing behavior.

#### Scenario: IPC mode enabled by default
- **WHEN** the user starts the tool without `--no-ipc`
- **THEN** the tool loads mappings, spawns the IPC thread, and populates the value table
- **THEN** the TUI indicates IPC mode in the table title
- **THEN** FSUIPC clients can connect and read evaluated offset values

#### Scenario: Offline mode via --no-ipc
- **WHEN** the user starts the tool with `--no-ipc`
- **THEN** all existing offline behavior (mapping evaluation, TUI display, CSV I/O) SHALL work identically to before
- **THEN** the TUI indicates offline mode in the table title

#### Scenario: Graceful fallback on IPC failure
- **WHEN** IPC initialization fails (e.g., window class registration fails)
- **THEN** the tool logs the error and falls back to offline mode
- **THEN** the TUI indicates offline mode in the table title

### Requirement: IPC value table population
In IPC mode, the tool SHALL periodically evaluate all mappings using `EvalEngine::evaluate_all()` and populate the IPC value table via `ipc_host::value_table::set_value_table()`. Each mapping's FSUIPC value SHALL be inserted at its offset with the correct FSUIPC type. The update rate SHALL be approximately 20Hz (every 50ms).

#### Scenario: Table populated on first tick
- **WHEN** IPC mode starts and the first evaluation tick occurs
- **THEN** the value table contains one entry per mapping, keyed by offset, with correct FSUIPC type values
- **THEN** FSUIPC clients can read those values via the IPC shared memory

#### Scenario: Table updated on state change
- **WHEN** the state HashMap is updated (e.g., via state reload)
- **THEN** the next evaluation tick updates the value table with new computed values
- **THEN** FSUIPC clients see the updated values

### Requirement: Value type conversion
The tool SHALL convert `EvalEngine` f64 results to the correct `ipc_host::value_table::Value` variant based on the mapping's `FsuipcType`, matching the conversion logic in `xplane_uipc::PluginState::update()`. Writable mappings SHALL have `writable: true` in the table entry.

#### Scenario: Type conversion for each FSUIPC type
- **WHEN** a mapping has `FsuipcType::U16` and evaluates to `250.0`
- **THEN** the value table stores `Value::UnsignedInt16(250)` at the mapping's offset

#### Scenario: Writable flag preserved
- **WHEN** a mapping has `writable: true`
- **THEN** the value table entry has `writable: true`

### Requirement: TUI mode indicator
The TUI SHALL display a visual indicator showing the current mode. In IPC mode (default), the offset table title SHALL show ` Mappings (IPC) ` in green. In offline mode (`--no-ipc`), the title SHALL show ` Mappings (Offline) ` in default styling.

#### Scenario: Title shows IPC in default mode
- **WHEN** the tool is started without `--no-ipc`
- **THEN** the offset table pane shows title ` Mappings (IPC) ` in green

#### Scenario: Title shows Offline in --no-ipc mode
- **WHEN** the tool is started with `--no-ipc`
- **THEN** the offset table pane shows title ` Mappings (Offline) ` with default styling
