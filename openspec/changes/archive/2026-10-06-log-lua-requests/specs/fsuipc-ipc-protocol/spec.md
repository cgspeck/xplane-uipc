## MODIFIED Requirements

### Requirement: Application key write at 0x8001

A write to offset `0x8001` SHALL be treated as an application key registration. It SHALL NOT be forwarded, SHALL NOT produce non-active/non-writable warnings, and SHALL NOT count as an error. The server SHALL log the key text (printable ASCII up to the first NUL, limited to `nBytes`) at the level set by `key_write` in `config.toml`'s `[log_levels]` table. Accepted values are `off`, `error`, `warn`, `info`, `debug` and `trace`. A missing setting SHALL mean `info`. An invalid value SHALL log a warning and fall back to `info`. The setting SHALL be re-applied when the config is reloaded.

The older `[settings] key_write_log_level` SHALL still be read as an alias. If both are set, `[log_levels] key_write` SHALL win and a deprecation warning SHALL be logged.

#### Scenario: Key write logged
- **WHEN** a client writes the 13 bytes `"6PETEXPDRVW3\0"` to `0x8001` and `[log_levels] key_write` is `info`
- **THEN** an info-level log line containing `6PETEXPDRVW3` is emitted, and no write is forwarded

#### Scenario: Key write silenced
- **WHEN** `[log_levels] key_write = "off"`
- **THEN** the same write produces no log line

#### Scenario: Old setting still honoured
- **WHEN** `config.toml` has `[settings] key_write_log_level = "off"` and no `[log_levels] key_write`
- **THEN** key writes produce no log line

#### Scenario: New setting wins
- **WHEN** `config.toml` has both `[settings] key_write_log_level = "off"` and `[log_levels] key_write = "info"`
- **THEN** key writes are logged at info, and a deprecation warning about `key_write_log_level` is logged when the config loads
