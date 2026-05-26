### Requirement: Load mapping file
The tool SHALL accept a path to a TOML mapping file (the existing `mappings.toml` format used by `uipc-mapping`) via the `--mapping` CLI argument. The tool SHALL load and parse the file using the existing `uipc-mapping::load_mappings()` function.

#### Scenario: Load valid mapping file
- **WHEN** the user starts the tool with `--mapping mappings.toml`
- **THEN** the tool loads and parses the file, displaying all successful mappings in the offset table pane

#### Scenario: Load mapping file with parse errors
- **WHEN** the mapping file contains invalid TOML or missing required fields
- **THEN** the tool displays the error in the trace log pane and exits with a non-zero code

### Requirement: Load state CSV
The tool SHALL accept an optional path to a state CSV file via the `--state` CLI argument. The CSV SHALL be headerless with two columns: `dataref_path,value`. The tool SHALL parse each line and store the key-value pair in an in-memory map.

#### Scenario: Load state CSV successfully
- **WHEN** the user provides `--state state.csv` containing `sim/flightmodel/position/indicated_airspeed,250.0`
- **THEN** the tool stores `{"sim/flightmodel/position/indicated_airspeed": 250.0}` and uses it for mapping evaluation

#### Scenario: Load state CSV with invalid number
- **WHEN** a line has a non-numeric value after the comma
- **THEN** the tool logs the error to the trace log pane and skips that line

#### Scenario: No state file provided
- **WHEN** the user starts with only `--mapping mappings.toml`
- **THEN** all mappings show "(not in state)" and no FSUIPC values are computed

### Requirement: Evaluate simple mappings
For mappings with a single dataref (`MappingSource::Simple`), the tool SHALL look up the dataref path in the state map. If found, it SHALL compute `fsuipc = value * scale + offset_add` and display the result.

#### Scenario: Simple mapping with state value
- **WHEN** a mapping has `dataref = "sim/flightmodel/position/indicated_airspeed"`, `scale = 128.0`, and the state has `sim/flightmodel/position/indicated_airspeed,250.0`
- **THEN** the table shows FSUIPC value `32000`

#### Scenario: Simple mapping missing from state
- **WHEN** a mapping's dataref path is not found in the state map
- **THEN** the table shows `—` for the FSUIPC value and the trace log records a missing-key warning

### Requirement: Evaluate expression mappings
For mappings with an RPN expression (`MappingSource::Expr`), the tool SHALL resolve each `(name, path)` pair in the `datarefs` map by looking up `path` in the state CSV. The resulting name-value map SHALL be passed to `Expr::eval()`. The result SHALL be displayed as the FSUIPC value.

#### Scenario: Expression mapping with all variables present
- **WHEN** an expression mapping has `datarefs = { Nav = "...", Bcn = "..." }`, `expr = "$Nav 1 * $Bcn 2 * +"`, and the state has both dataref paths
- **THEN** the tool evaluates the RPN and displays the computed FSUIPC value

#### Scenario: Expression mapping missing a variable
- **WHEN** a dataref path referenced by the expression is missing from the state
- **THEN** the tool uses `0.0` as the value for that variable (matching `Expr::eval` default behavior) and logs a warning to the trace log

### Requirement: Evaluate static mappings
For mappings with a static value (`MappingSource::Static`), the tool SHALL display the static value directly as the FSUIPC value. No state lookup is required.

#### Scenario: Static mapping displayed
- **WHEN** a mapping has `static_value = 0x50000008`
- **THEN** the table shows `0x50000008` as the FSUIPC value

### Requirement: Display mapping table
The table pane SHALL display one row per mapping with columns: offset (hex), FSUIPC type, writable flag, input dataref values, computed FSUIPC value, and source summary (expression string or `path * scale`).

#### Scenario: Table populated on load
- **WHEN** mappings and state are loaded
- **THEN** each mapping occupies one row in the table with all columns populated

### Requirement: Expression detail popup
When a row with an expression source is selected and Enter is pressed, a popup SHALL display: the full RPN expression string and a table of each variable name, its resolved dataref path, its state value, and its type.

#### Scenario: Open expression detail
- **WHEN** the user selects a row with an expression source and presses Enter
- **THEN** a popup overlay displays the expression details
- **WHEN** the user presses Esc
- **THEN** the popup closes and focus returns to the table
