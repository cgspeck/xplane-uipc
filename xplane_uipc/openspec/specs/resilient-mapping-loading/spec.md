## Requirements

### Requirement: Per-mapping error isolation
The mapping loader SHALL process each mapping independently so that a failure in one mapping does not prevent other valid mappings from loading. Each mapping's parse and validation is attempted regardless of other mappings' outcomes.

#### Scenario: Single mapping fails, others succeed
- **WHEN** a mappings.toml file contains 5 valid mappings and 1 invalid mapping (e.g., bad expression)
- **THEN** the 5 valid mappings are loaded successfully and the invalid mapping is skipped

#### Scenario: All mappings fail
- **WHEN** a mappings.toml file contains only invalid mappings
- **THEN** the load operation returns an error indicating no mappings could be loaded

### Requirement: Per-mapping error logging with tracing::error!
For each mapping that fails to load or parse, the system SHALL log an error using `tracing::error!` that includes the FSUIPC offset and a description of the error.

#### Scenario: Expression parse error is logged
- **WHEN** a mapping has an invalid RPN expression
- **THEN** `tracing::error!` is called with a message containing the offset (e.g., `0x07BC`) and the parse error details

#### Scenario: Missing source fields error is logged
- **WHEN** a mapping has no `dataref`, `expr`, or `static_value` field
- **THEN** `tracing::error!` is called with a message containing the offset and describing the missing field requirement

### Requirement: Unrecoverable errors still fail the load
The system SHALL return an error for unrecoverable conditions: mapping file not found, unable to parse the TOML file, or no mapping definitions found in the file.

#### Scenario: File not found returns error
- **WHEN** the mappings.toml file does not exist at the expected path
- **THEN** `load_mappings()` returns `Err` with a message indicating the file was not found

#### Scenario: Invalid TOML returns error
- **WHEN** the mappings.toml file contains syntactically invalid TOML
- **THEN** `load_mappings()` returns `Err` with a message indicating the TOML parse error

#### Scenario: Empty mappings list returns error
- **WHEN** the mappings.toml file parses successfully but contains no mapping entries (or all mappings failed)
- **THEN** `load_mappings()` returns `Err` indicating no mappings could be loaded

### Requirement: Load result includes error summary
The `MappingConfig` returned on successful (including partial-success) load SHALL include a list of per-mapping errors that occurred during loading, so callers can log a summary.

#### Scenario: Partial success includes error list
- **WHEN** some mappings fail but at least one succeeds
- **THEN** `MappingConfig` contains the successfully loaded mappings and a non-empty `load_errors` list with details of each failure

#### Scenario: Full success has empty error list
- **WHEN** all mappings load successfully
- **THEN** `MappingConfig` contains all mappings and an empty `load_errors` list

### Requirement: Caller logs partial-success summary
The caller (`load_mappings_and_init`) SHALL log a summary when mappings are loaded with errors, using `tracing::error!` to report the count of successful and failed mappings followed by each individual error.

#### Scenario: Partial success logs summary and details
- **WHEN** `load_mappings()` returns `Ok(config)` with non-empty `load_errors`
- **THEN** the caller logs `"Loaded N mappings with M errors"` followed by each error message
