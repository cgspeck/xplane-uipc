## ADDED Requirements

### Requirement: In-memory trace log
The tool SHALL maintain a ring-buffer of trace log entries in memory. The last N entries (configurable, default 1000) SHALL be displayed in the trace log pane. Each entry SHALL include a timestamp and message.

#### Scenario: Trace entries appear in pane
- **WHEN** the tool loads a mapping file or state CSV
- **THEN** a timestamped entry appears in the trace log pane describing the action and result

#### Scenario: Ring buffer truncation
- **WHEN** the number of trace entries exceeds the configured maximum
- **THEN** the oldest entries are discarded

### Requirement: Trace log auto-scroll
The trace log pane SHALL auto-scroll to show the most recent entry at the bottom. When the user manually scrolls up, auto-scroll SHALL pause. Pressing a key (e.g. End) SHALL resume auto-scroll.

#### Scenario: Auto-scroll on new entry
- **WHEN** a new trace entry is added
- **THEN** the pane scrolls to show the newest entry
- **WHEN** the user scrolls up
- **THEN** auto-scroll pauses
- **WHEN** the user presses End
- **THEN** auto-scroll resumes

### Requirement: Default file logging
The tool SHALL write all trace entries to `uipc-debug.log` in the current directory by default. The path SHALL be overridable with `--log-file <PATH>`. File logging SHALL be disabled entirely with `--no-log-file`.

#### Scenario: Default file logging
- **WHEN** the user starts the tool without `--log-file` or `--no-log-file`
- **THEN** all trace entries are written to `uipc-debug.log` in the current directory

#### Scenario: Custom log file path
- **WHEN** the user provides `--log-file debug.log`
- **THEN** all trace entries are written to `debug.log` in addition to the in-memory buffer

#### Scenario: File logging disabled
- **WHEN** the user provides `--no-log-file`
- **THEN** trace entries are in-memory only, no file is written

### Requirement: Configurable log level
The tool SHALL default to `TRACE` log level. The level SHALL be overridable with `--log-level <LEVEL>` accepting: `trace`, `debug`, `info`, `warn`, `error`.

#### Scenario: Default TRACE level
- **WHEN** the user starts the tool without `--log-level`
- **THEN** all trace-level events are captured and displayed

#### Scenario: Override log level
- **WHEN** the user provides `--log-level info`
- **THEN** only INFO and above events are captured; DEBUG and TRACE events are suppressed

### Requirement: Toggleable log pane
The trace log pane SHALL be hidden and shown by pressing `l`. When hidden, the offset table pane SHALL expand to fill the full terminal height. The ring buffer SHALL continue accepting entries while hidden.

### Requirement: Scrollable log pane
When the log pane has focus, `Page Up` and `Page Down` SHALL scroll the trace log content. `End` SHALL resume auto-scroll to the latest entry. When focus is on the table pane, these keys SHALL have no effect on the log pane.

#### Scenario: Scroll log pane
- **WHEN** the log pane has focus and the user presses Page Down
- **THEN** the log pane scrolls forward one page
- **WHEN** the user presses Page Up
- **THEN** the log pane scrolls backward one page
- **WHEN** auto-scroll is paused and the user presses End
- **THEN** auto-scroll resumes, jumping to the latest entry

### Requirement: Hide log pane
- **WHEN** the user presses `l`
- **THEN** the trace log pane disappears and the table pane expands to fill the terminal
- **WHEN** the user presses `l` again
- **THEN** the trace log pane reappears at its previous size
