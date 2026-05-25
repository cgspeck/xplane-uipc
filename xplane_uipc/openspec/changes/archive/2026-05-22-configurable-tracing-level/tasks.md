## 1. Subscriber initialization with reload support

- [x] 1.1 In `src/lib.rs`, wrap the `LevelFilter` in `tracing_subscriber::reload::Layer` and store the `reload::Handle` in a global `OnceLock<reload::Handle<LevelFilter, impl Layer<...>>>`
- [x] 1.2 Use the stored handle to inject the filter into the subscriber registry via `.with(reloaded_layer)`

## 2. Config reload helper

- [x] 2.1 Create a `reload_config_and_apply()` function in `src/lib.rs` that reads `config.toml` via `toml::from_str`, extracts `log_level` from `[settings]`, parses it with `LevelFilter::from_str`, falls back to `INFO` on any error, and calls `handle.reload(new_filter)`

## 3. Wire config reload into startup

- [x] 3.1 Call `reload_config_and_apply()` at the end of `load_mappings_and_init()` so the filter is applied on first load (after subscriber is already initialized)

## 4. Wire config reload into menu reload path

- [x] 4.1 In `menu.rs`, ensure the existing `crate::load_mappings_and_init()` call already triggers the config reload (since the call goes through step 3)

## 5. Update config.toml

- [x] 5.1 Add `log_level = "info"` to the `[settings]` section in `config.toml`

## 6. Verify

- [x] 6.1 Build the plugin and confirm it compiles
- [ ] 6.2 Test with `log_level = "debug"` in config.toml — verify DEBUG messages appear in the log file
- [ ] 6.3 Test with invalid `log_level` value — verify fallback to INFO and a warning is logged
- [ ] 6.4 Test log level change + reload — verify the filter switches dynamically
