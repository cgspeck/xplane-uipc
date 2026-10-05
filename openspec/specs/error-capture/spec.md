## ADDED Requirements

### Requirement: Capture configuration at startup

`create_ipc_window_and_run` SHALL accept a `CaptureConfig` struct with an optional capture path and optional max capture count.

- **WHEN** `capture_path` is `None`, capture SHALL be unavailable (no captures possible even if started)
- **WHEN** `max_captures` is `None`, there SHALL be no file count limit
- **WHEN** `max_captures` is `Some(n)`, the system SHALL stop capturing after `n` files and log a warning

#### Scenario: No capture path provided
- **WHEN** `create_ipc_window_and_run` is called with `capture_path: None`
- **THEN** capture operations SHALL be a no-op even if `StartCapture` is sent

#### Scenario: Max captures reached
- **WHEN** `max_captures` is `Some(1000)` and 1000 files have been written
- **THEN** capture SHALL stop, `enabled` SHALL be set to `false`, and a warning SHALL be logged

### Requirement: Runtime capture toggling

The system SHALL expose two new `IpcCommands` variants to enable and disable capture at runtime.

- `StartCapture` SHALL set the capture state to enabled
- `StopCapture` SHALL set the capture state to disabled
- Capture state SHALL persist in a `CaptureState` struct behind a global `Mutex`

#### Scenario: Start and stop capture
- **WHEN** `IpcCommands::StartCapture` is received
- **THEN** subsequent mapped views with errors SHALL be captured
- **WHEN** `IpcCommands::StopCapture` is received
- **THEN** subsequent mapped views with errors SHALL NOT be captured

### Requirement: Capture triggers

The system SHALL capture a mapped view's raw bytes only when all of the following are true:
1. Capture is enabled (`StartCapture` was sent)
2. `capture_file_count < max_captures` (if a limit is configured)
3. The view is not zero bytes

#### Scenario: Capture of a malformed view
- **WHEN** a mapped view contains a record with an unknown `dwId`
- **AND** capture is enabled
- **THEN** the raw bytes of the entire mapped view SHALL be written to a `.bin` file

#### Scenario: Capture of a well-formed view
- **WHEN** capture is enabled and a well-formed view arrives
- **THEN** the raw bytes of the entire mapped view SHALL be written to a `.bin` file

### Requirement: Capture file format

Each capture file SHALL be named `<timestamp>.bin` where timestamp is ISO 8601 with millisecond precision (format `%Y-%m-%dT%H-%M-%S.%3fZ`).

If two messages arrive within the same millisecond, a counter SHALL be appended: `<timestamp>_<n>.bin`.

The file SHALL contain the exact raw bytes of the mapped view, as received from the client, before any processing.

#### Scenario: File naming with conflict
- **WHEN** two messages arrive at the same millisecond
- **THEN** the first SHALL be `2026-05-22T10-30-45.123Z.bin`
- **THEN** the second SHALL be `2026-05-22T10-30-45.123Z_1.bin`

### Requirement: Directory creation

If the capture path does not exist, the system SHALL create it (recursively) at initialization and log an info message.

#### Scenario: Missing directory
- **WHEN** the capture path does not exist
- **THEN** the system SHALL create it with `create_dir_all`
- **AND** log an info-level message

### Requirement: View size via VirtualQuery

The system SHALL use `VirtualQuery` on the mapped view address to determine its size for capture.

#### Scenario: View size determined
- **WHEN** a mapped view is opened and capture is needed
- **THEN** the system SHALL call `VirtualQuery` to get `RegionSize`
- **AND** write exactly `RegionSize` bytes to the capture file

### Requirement: Resilient parsing context logging

When a view is malformed, the system SHALL log the byte offset where parsing stopped, the reason, and whatever header fields could be read (`dwId`, `dwOffset`, `nBytes`), so the operator can identify the failing record.

#### Scenario: Malformed record logged
- **WHEN** parsing stops on an unknown `dwId`
- **THEN** a warning SHALL include the byte offset and `dwId={val}`
