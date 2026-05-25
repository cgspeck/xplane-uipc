## Why

The tracing level filter is hard-coded to `LevelFilter::INFO` at `src/lib.rs:135`, making it impossible to adjust logging verbosity without recompiling. This makes debugging production issues harder than necessary — users must rebuild the plugin to get DEBUG or TRACE output.

## What Changes

- Add a `log_level` setting to `config.toml` under `[settings]` (e.g., `log_level = "debug"`)
- Parse this setting at startup and use it to configure the tracing subscriber's level filter
- When config.toml cannot be parsed or the level value is invalid, default to `INFO`
- During mapping reloads, also reload config.toml and update the subscriber's level filter dynamically

## Capabilities

### New Capabilities
- `configurable-log-level`: The plugin reads its log level from config.toml and applies it to the tracing subscriber, with dynamic updates on reload

### Modified Capabilities
*(none — no existing spec changes)*

## Impact

- `src/lib.rs`: Replace the hard-coded `LevelFilter::INFO` with a value parsed from config.toml
- `src/plugin_state.rs` or new module: Add config reload logic that re-reads config.toml and updates the subscriber filter
- `config.toml`: Add `log_level` field under `[settings]`
- `menu.rs`: Mapping reload path will also trigger config reload (which includes log level update)
- New dependency: `tracing-subscriber` is already used; no new crate needed for level parsing (can parse string directly)
