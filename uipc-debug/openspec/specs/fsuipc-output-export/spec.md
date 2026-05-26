### Requirement: Write computed FSUIPC values to CSV
The tool SHALL write the computed FSUIPC values to a CSV file when the user presses `c`. The output SHALL contain one row per mapping with columns: `offset` (hex), `type` (FSUIPC type string), `value` (computed numeric value), `writable` (true/false).

#### Scenario: Write computed values with all mappings evaluated
- **WHEN** the user presses `c` and all mappings have computed values
- **THEN** a CSV file is written with offset, type, and value for every mapping

#### Scenario: Write computed values with missing state entries
- **WHEN** some mappings could not be evaluated due to missing state keys
- **THEN** the CSV still includes those mappings with value `0` and a note logged to the trace pane

#### Scenario: Prompt for output path
- **WHEN** the user presses `c`
- **THEN** the tool prompts for the output CSV file path
