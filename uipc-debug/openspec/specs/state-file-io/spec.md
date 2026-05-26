### Requirement: Write state CSV
The tool SHALL write the current in-memory state to a CSV file when the user presses `w`. The output SHALL include all dataref keys referenced by the loaded mappings. Keys that exist in the current state retain their values; keys not in the current state SHALL be written with value `0.0`.

#### Scenario: Write state with all keys present
- **WHEN** the user presses `w` and all mapping-referenced datarefs are in the current state
- **THEN** a CSV file is written with all keys and their current values

#### Scenario: Write state with missing keys filled
- **WHEN** a mapping references a dataref not present in the current state
- **THEN** the written CSV includes that key with value `0.0`

#### Scenario: Prompt for output path
- **WHEN** the user presses `w`
- **THEN** the tool prompts for the output file path (defaulting to the loaded state path or a generated name)

### Requirement: Reload state CSV
The tool SHALL reload the state CSV from disk when the user presses `l`, replacing the current in-memory state. The user SHALL be prompted for the file path.

#### Scenario: Reload state
- **WHEN** the user presses `l` and enters a valid file path
- **THEN** the state is replaced and the table re-evaluates all mappings
- **WHEN** the new state has different values
- **THEN** the table updates to reflect the new computed FSUIPC values

#### Scenario: Reload from non-existent file
- **WHEN** the entered path does not exist
- **THEN** the tool logs an error in the trace pane and keeps the current state unchanged
