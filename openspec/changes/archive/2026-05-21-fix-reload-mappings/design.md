## Context

Currently there are two paths that load the mappings file:

1. **Startup** (`lib.rs:load_mappings_and_init`): Constructs an absolute path using `XPLMGetSystemPath` + `Resources/plugins/xplane-uipc/mappings.toml`, calls `mapping::load_mappings`, then converts each entry into a `ResolvedMapping`.

2. **Reload Mappings** (`plugin_state.rs:reload_mappings`): Calls `mapping::load_mappings("mappings.toml")` with a hardcoded relative path, then duplicates the `ResolvedMapping::new` conversion. The relative path resolves to the process working directory (usually the X-Plane root), not the plugin directory.

This means reload always fails or loads the wrong file, and any future changes to the mapping resolution pipeline must be made in two places.

## Goals / Non-Goals

**Goals:**
- The "Reload Mappings" menu item uses the same mapping-loading function as startup
- Both paths resolve `mappings.toml` using the same absolute path construction
- Eliminate code duplication in the mapping resolution pipeline
- Add the "Reload Mappings" menu item if it doesn't already exist

**Non-Goals:**
- Changing the mapping file format or schema
- Adding new mapping sources or expression features
- Persisting mapping state across reloads (the table is fully rebuilt on reload)

## Decisions

1. **Extract a shared `load_mappings_from_plugin_dir` function** from `load_mappings_and_init`:
   - Takes no arguments, calls `XPLMGetSystemPath` internally, constructs the absolute path to `mappings.toml`, calls `mapping::load_mappings`, and resolves each entry to `ResolvedMapping`.
   - Returns `Vec<ResolvedMapping>`.
   - Both `load_mappings_and_init` (startup) and `reload_mappings` (menu handler) call this function.

2. **`reload_mappings` delegates fully**: Instead of calling `mapping::load_mappings` directly with a relative path, it calls the same shared function used at startup. The `PluginState` is updated in-place with the new mappings.

3. **Single source of truth for the mappings path**: The absolute path construction lives in one place. If the plugin directory structure changes, only one function needs updating.

4. **Error handling**: If reload fails (file not found, parse error, etc.), the old mappings remain in place and an error is logged. The plugin continues running with the previous mappings.

## Risks / Trade-offs

- **[Risk]** `XPLMGetSystemPath` must be called on the main X-Plane thread. If the reload handler runs on a different thread, it could crash.
  - **Mitigation**: The menu handler runs on the main X-Plane thread in practice. If threading becomes an issue, the path can be captured once at startup and stored.

- **[Risk]** Large mappings file could cause a brief stutter on reload.
  - **Mitigation**: Acceptable — reload is a user-initiated action, not a frequent operation. The flight loop continues unaffected during reload.
