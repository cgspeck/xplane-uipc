## MODIFIED Requirements

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

### Requirement: Resilient parsing context logging

When a view is malformed, the system SHALL log the byte offset where parsing stopped, the reason, and whatever header fields could be read (`dwId`, `dwOffset`, `nBytes`), so the operator can identify the failing record.

#### Scenario: Malformed record logged
- **WHEN** parsing stops on an unknown `dwId`
- **THEN** a warning SHALL include the byte offset and `dwId={val}`

## REMOVED Requirements

### Requirement: Resilient parsing

**Reason**: Without a sentinel, nothing reliably marks where the next record starts after a misframed one. Scanning forward produced junk records. The SDK client also stops at an unknown `dwId`.
**Migration**: Parsing stops at the first malformed record, counts one error, and the client is told via `FS6IPC_MESSAGE_FAILURE` (see `fsuipc-ipc-protocol`).
