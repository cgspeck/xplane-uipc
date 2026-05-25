## ADDED Requirements

### Requirement: User can clear the trace log from the plugin menu

The system SHALL provide a "Clear Trace Log" item in the X-Plane UIPC plugin menu. When activated, the system SHALL flush the current log file, truncate it to zero bytes, write a "Log file cleared" marker line, and continue writing new trace output to the cleared file.

#### Scenario: Menu item clears the log
- **WHEN** the user selects "Clear Trace Log" from the X-Plane UIPC plugin menu
- **THEN** `uipc.log` is truncated to zero bytes
- **AND** a line `Log file cleared` is written at the top of the new empty file
- **AND** subsequent trace output continues to write to the same file

#### Scenario: Flush before truncation
- **WHEN** "Clear Trace Log" is activated
- **THEN** any buffered trace output is flushed to the file before truncation

### Requirement: Tracing continues after clear without interruption

After the log is cleared, all trace events (`trace!`, `debug!`, `info!`, `warn!`, `error!`) SHALL continue to be written to `uipc.log` at the same log level as before the clear.

#### Scenario: Trace output after clear
- **WHEN** the log has been cleared via the menu item
- **AND** a `tracing::info!("test message")` macro is invoked
- **THEN** the message appears in the new, cleared `uipc.log` file

### Requirement: Clear is safe under concurrent tracing

The clear operation SHALL NOT corrupt the log file when trace events are being written concurrently by other threads.

#### Scenario: Concurrent clear and trace
- **WHEN** trace events are being written continuously
- **WHEN** "Clear Trace Log" is activated simultaneously
- **THEN** the log file SHALL remain valid and readable
- **AND** no data loss SHALL occur beyond the truncation boundary

### Requirement: Clear resets all deduplicated warning flags

"Clear Trace Log" SHALL also reset all warning deduplication state (`WarnedSet`), so that suppressed warnings for missing or non-writable offsets can fire again after the clear.

#### Scenario: Warnings re-fire after clear
- **WHEN** offset `0x3304` is not in the table
- **AND** an IPC client reads `0x3304` (a `ReadNotExist` warning fires once, then is suppressed)
- **WHEN** the user selects "Clear Trace Log" from the plugin menu
- **AND** the IPC client reads `0x3304` again
- **THEN** a `ReadNotExist` warning is emitted again

#### Scenario: Clear deduplication is independent of file truncation
- **WHEN** warnings have been suppressed by `WarnedSet`
- **WHEN** "Clear Trace Log" is activated
- **THEN** warning flags are reset regardless of whether the log file truncation succeeds
