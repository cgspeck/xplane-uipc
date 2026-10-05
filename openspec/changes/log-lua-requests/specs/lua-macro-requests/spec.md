## ADDED Requirements

### Requirement: Lua and macro parameter at 0x0D6C

A write to `0x0D6C` SHALL be stored as the macro/Lua parameter: its first four bytes (or fewer, if fewer were written) read as a little-endian unsigned integer, zero-extended. The write SHALL NOT be forwarded, SHALL NOT produce non-active/non-writable warnings, and SHALL NOT count as an error. The stored parameter SHALL persist across messages until the next write to `0x0D6C`, and SHALL NOT be cleared when it is used.

#### Scenario: Parameter stored
- **WHEN** a client writes the 4 bytes `03 00 00 00` to `0x0D6C`
- **THEN** no write is forwarded, nothing is warned about, and the stored parameter is `3`

### Requirement: Lua and macro request at 0x0D70

A write to `0x0D70` SHALL be treated as a request for FSUIPC to run a macro or Lua control. The server SHALL log the request text (printable ASCII up to the first NUL, limited to `nBytes` and to 40 bytes, with other bytes escaped) together with the stored parameter, or "no param" if none has been written, and SHALL say the request is not supported. A request with the same text and parameter as one already logged SHALL NOT be logged again at that level until warnings are reset by "Clear Trace Log". The line SHALL be logged at the level set by `[log_levels] lua_request` in `config.toml`, which accepts the same values and defaults as `[log_levels] key_write`. The write SHALL NOT be forwarded, SHALL NOT produce non-active/non-writable warnings, and SHALL NOT count as an error.

#### Scenario: Request with a parameter
- **WHEN** a client writes `3` to `0x0D6C` and then `"LuaSet slc_doors\0"` to `0x0D70`, with `lua_request = "info"`
- **THEN** an info-level log line contains `LuaSet slc_doors` and `param 3`, and says the request is not supported

#### Scenario: Request without a parameter
- **WHEN** no parameter has been written since the plugin started and a client writes `"LuaKill slc_doors\0"` to `0x0D70`
- **THEN** the log line contains `LuaKill slc_doors` and `no param`

#### Scenario: Parameter reused
- **WHEN** a client writes `5` to `0x0D6C`, then writes two different requests to `0x0D70` in later messages
- **THEN** both log lines contain `param 5`

#### Scenario: Repeated request logged once
- **WHEN** a client writes `"LuaSet slc_doors\0"` to `0x0D70` in several messages without changing the parameter
- **THEN** only one log line is written at the `lua_request` level

#### Scenario: Same text with a new parameter
- **WHEN** a client writes `"LuaSet slc_doors\0"` to `0x0D70` with parameter `3`, then writes `5` to `0x0D6C` and the same request again
- **THEN** two log lines are written, one with `param 3` and one with `param 5`

#### Scenario: Logged again after reset
- **WHEN** a request has been logged and the user selects "Clear Trace Log"
- **THEN** the next write of the same request is logged again

#### Scenario: Requests silenced
- **WHEN** `[log_levels] lua_request = "off"`
- **THEN** writes to `0x0D70` produce no log line and no warning

### Requirement: Lua and macro offsets read as zero

Reads of `0x0D6C` and `0x0D70` SHALL be answered with zero bytes, like any other offset that isn't served.

#### Scenario: Reading the request area
- **WHEN** a client reads 40 bytes from `0x0D70` after writing a request
- **THEN** the payload is 40 zero bytes
