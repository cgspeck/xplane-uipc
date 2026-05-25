## Context

The tracing subscriber is initialized once during `XPluginStart` with a hard-coded `LevelFilter::INFO`. The subscriber's layer stack is built inline and `.init()` consumes the registry, making it impossible to change the filter after initialization without using `tracing_subscriber::reload`.

Config.toml is already loaded during `load_mappings_and_init()`, but the log level setting is not read. The existing config struct is embedded inside `PluginState` but only `update_rate_hz` is used.

## Goals / Non-Goals

**Goals:**
- Allow users to set `log_level` in config.toml to control tracing verbosity
- Default to `INFO` when config is missing, unparseable, or the level value is invalid
- Support dynamic reload: when mappings are reloaded via the menu, also reload config.toml and update the subscriber's level filter

**Non-Goals:**
- Per-module log level filtering (e.g., `warn` for one module, `debug` for another)
- Runtime log level changes outside the reload flow (e.g., CLI or keybinding)
- Validation or error reporting for invalid log_level values beyond falling back to INFO

## Decisions

1. **Use `tracing_subscriber::reload` for dynamic filtering**
   - Wrap the `LevelFilter` in a `reload::Layer` so the handle can be stored and used to swap filters later
   - Store the reload handle in a global `static` (via `LazyLock` or `OnceLock`) accessible from `load_mappings_and_init()`
   - **Alternative considered**: Re-init the subscriber — not possible, `tracing` does not support re-initialization

2. **Parse `log_level` as a string, map via `FromStr` on `LevelFilter`**
   - `LevelFilter` already implements `FromStr` accepting: `"off"`, `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"` (case-insensitive)
   - On parse failure, log a warning and fall back to `LevelFilter::INFO`
   - No new dependencies needed

3. **Config reload lives in a shared helper function, not in PluginState**
   - `PluginState` already stores `config_path` but the reload of config is conceptually separate from FSUIPC mapping state
   - A free function `reload_config_and_apply(path) -> Result<(), String>` handles parsing config.toml and updating the subscriber
   - Called from both the init path and the menu reload path

4. **Config.toml layout**
   ```toml
   [settings]
   update_rate_hz = 20
   log_level = "info"
   ```
   - `log_level` is optional — missing key defaults to `"info"` in the parser

## Risks / Trade-offs

- **[Low] Global reload handle**: Using a global static for the reload handle is safe because `LevelFilter` is `Send + Sync` and the handle is only written once (before any reload call) and read from the reload path. A `OnceLock` ensures single initialization.
- **[Low] Race on subscriber init**: The reload handle must be stored after the subscriber is initialized but before any reload can occur. Since both happen in `XPluginStart` → `load_mappings_and_init()` on the same thread, this is safe.
