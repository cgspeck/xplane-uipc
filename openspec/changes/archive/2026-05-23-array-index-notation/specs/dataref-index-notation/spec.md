## ADDED Requirements

### Requirement: Bracket notation for array index in simple mappings

A simple `[[mapping]]` entry SHALL support specifying the array index via bracket notation in the `dataref` field: `dataref = "path[N]"` where `N` is an integer. The system SHALL strip the `[N]` suffix before passing the path to `XPLMFindDataRef` and SHALL use `N` as the array offset in `XPLMGetDatavi`/`XPLMGetDatavf`.

#### Scenario: Simple mapping with bracket index
- **WHEN** a mapping has `dataref = "sim/flightmodel/engine/ENGN_N1_[0]"`
- **THEN** the system SHALL call `XPLMFindDataRef` with `"sim/flightmodel/engine/ENGN_N1_"`
- **THEN** reading the dataref SHALL call `XPLMGetDatavi`/`XPLMGetDatavf` with offset `0`

#### Scenario: Simple mapping without bracket index (scalar)
- **WHEN** a mapping has `dataref = "sim/flightmodel/position/indicated_airspeed"` (no brackets)
- **THEN** the system SHALL call `XPLMFindDataRef` with the path as-is
- **THEN** reading the dataref SHALL use scalar `XPLMGetDataf`/`XPLMGetDatai` (not array getters)

### Requirement: No standalone `array_index` field

The `array_index` TOML field SHALL be removed from simple mapping entries. Any TOML file containing `array_index` SHALL produce a parse error.

#### Scenario: Old-style array_index rejected
- **WHEN** a mapping entry includes `array_index = 0`
- **THEN** the mapping loader SHALL reject the file with a TOML parse error

#### Scenario: Migration path
- **WHEN** a user previously wrote `dataref = "path"` + `array_index = N`
- **THEN** the user SHALL migrate to `dataref = "path[N]"`
