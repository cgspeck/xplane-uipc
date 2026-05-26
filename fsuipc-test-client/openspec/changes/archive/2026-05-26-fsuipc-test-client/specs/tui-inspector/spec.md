## ADDED Requirements

### Requirement: Display live offset table
The system SHALL display a table of all registered offsets with their current values, refreshed at approximately 1-2 Hz.

#### Scenario: Table shows address, type, and value
- **WHEN** the TUI is running
- **THEN** each row SHALL show the offset address (hex), type name, and current value

#### Scenario: Values update periodically
- **WHEN** the TUI has been running for several refresh cycles
- **THEN** displayed values SHALL reflect the latest data from `FSUIPCConnection.Process()`

### Requirement: Navigate rows
The system SHALL allow the user to navigate through the offset table using keyboard.

#### Scenario: Arrow key navigation
- **WHEN** the user presses `↑` or `↓`
- **THEN** the selected row SHALL move up or down by one

### Requirement: Reload input file
The system SHALL reload the offset definitions from the input file when the user presses `r`.

#### Scenario: Reload replaces all offsets
- **WHEN** the user presses `r`
- **THEN** the system SHALL re-read the input file, disconnect all existing offsets, and register new ones

### Requirement: Save snapshot
The system SHALL save the current offset values to a JSON file when the user presses `s`.

#### Scenario: Save writes JSON
- **WHEN** the user presses `s`
- **THEN** the system SHALL write a JSON file containing a timestamp and all offset address/value pairs

### Requirement: Quit
The system SHALL close the FSUIPC connection and exit when the user presses `q`.

#### Scenario: Clean quit
- **WHEN** the user presses `q`
- **THEN** the system SHALL call `FSUIPCConnection.Close()` and exit

### Requirement: Show help
The system SHALL display a help overlay listing all keybindings when the user presses `?`.

#### Scenario: Help overlay
- **WHEN** the user presses `?`
- **THEN** the system SHALL show a help panel with all available keybindings
