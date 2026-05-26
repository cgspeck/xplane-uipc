## ADDED Requirements

### Requirement: Batch query mode
The system SHALL support a batch mode that performs a single FSUIPC query and outputs results as JSON to stdout, then exits.

#### Scenario: Batch mode activates via flag
- **WHEN** the tool is invoked with `--batch` (or `-b`)
- **THEN** the system SHALL parse the input file, connect to FSUIPC, call `Process()` once, write JSON to stdout, close the connection, and exit

#### Scenario: JSON output format
- **WHEN** batch mode runs
- **THEN** the output SHALL be a JSON object with `timestamp` (ISO 8601) and `offsets` array containing `address`, `type`, and `value` for each offset

#### Scenario: Connection failure
- **WHEN** batch mode cannot connect to FSUIPC
- **THEN** the system SHALL write a JSON error response to stderr and exit with a non-zero code

### Requirement: TUI-batch parity
The JSON output format SHALL be identical between batch mode and the TUI save (`s`) function.

#### Scenario: Consistent schema
- **WHEN** comparing output from batch mode and TUI save
- **THEN** the JSON schema SHALL be identical in structure (same field names, same nesting)
