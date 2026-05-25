## Requirements

### Requirement: Log level is configurable via config.toml
The system SHALL read the log level from the `[settings]` section of config.toml using the key `log_level`. Valid values SHALL be `"off"`, `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"` (case-insensitive).

#### Scenario: Valid log level is set
- **WHEN** config.toml contains `[settings]\nlog_level = "debug"`
- **THEN** the tracing subscriber SHALL filter at DEBUG level

#### Scenario: Log level key is missing
- **WHEN** config.toml exists but has no `log_level` key under `[settings]`
- **THEN** the subscriber SHALL default to INFO level

#### Scenario: Config file cannot be parsed
- **WHEN** config.toml is missing, malformed, or unreadable
- **THEN** the subscriber SHALL default to INFO level without crashing

#### Scenario: Invalid log level value
- **WHEN** config.toml has `[settings]\nlog_level = "invalid_value"`
- **THEN** the subscriber SHALL default to INFO level and a warning SHALL be logged

### Requirement: Log level is updated on mapping reload
When mappings are reloaded via the plugin menu, the system SHALL also re-read config.toml and update the subscriber's level filter.

#### Scenario: Log level changed in config.toml then reload
- **WHEN** the user edits config.toml to change `log_level` from `"info"` to `"debug"` and selects "Reload Mappings" from the plugin menu
- **THEN** subsequent log messages at DEBUG level SHALL appear in the log file

#### Scenario: Invalid level survives reload
- **WHEN** `log_level` is changed to an invalid value and reload is triggered
- **THEN** the subscriber SHALL fall back to INFO and a warning SHALL be logged
