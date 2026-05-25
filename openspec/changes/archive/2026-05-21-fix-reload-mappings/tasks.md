## 1. Refactor `load_mappings_and_init` for Reuse

- [x] 1.1 Refactor `load_mappings_and_init()` in `lib.rs` to handle both startup and reload: constructs absolute path via `XPLMGetSystemPath`, calls `mapping::load_mappings`, resolves to `ResolvedMapping` vec; if `PLUGIN_STATE_PTR` is null, creates new `PluginState`; otherwise updates existing state's mappings in place
- [x] 1.2 Make `load_mappings_and_init()` public (`pub fn`) so it can be called from `menu.rs`
- [x] 1.3 `toml` and `serde` dependencies already present in `xplane-uipc/Cargo.toml`

## 2. Wire Refactored Loader into Startup

- [x] 2.1 In `lib.rs`, update `XPluginEnable` to call the refactored `load_mappings_and_init()` which now returns `Result<(), String>`
- [x] 2.2 Handle the error case — log failure but allow the plugin to continue with existing/empty state

## 3. "Reload Mappings" Menu Item

- [x] 3.1 `MENU_RELOAD` constant (value `1`) already exists in `menu.rs`
- [x] 3.2 `XPLMAppendMenuItem` call for "Reload Mappings" already exists in `build_menu()`
- [x] 3.3 Update `MENU_RELOAD` arm in `menu_handler()` to call `crate::load_mappings_and_init()` directly instead of `state.reload_mappings()`
- [x] 3.4 On successful reload, log "Mappings reloaded successfully"; on failure, log the error and preserve existing mappings (handled by `load_mappings_and_init` returning early on error)

## 4. Clean Up and Verify

- [x] 4.1 Remove `reload_mappings()` method from `PluginState` in `plugin_state.rs`
- [x] 4.2 Run `cargo check` to verify compilation
- [x] 4.3 Run `cargo test` (mapping tests exist in `mapping.rs`)
