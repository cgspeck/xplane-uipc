## Why

The "Reload Mappings" menu item has its own code to load the mappings file using a hardcoded relative path (`"mappings.toml"`), while startup uses a properly constructed absolute path via `XPLMGetSystemPath`. The reload code also duplicates the `mapping::load_mappings` → `ResolvedMapping::new` pipeline. This duplication means the reload fails at runtime (wrong path) and diverges from startup behavior.

## What Changes

- Extract the common mappings-loading logic (path construction + file parsing + resolution) into a shared function
- The startup path and the "Reload Mappings" handler both call the same shared function
- Ensure the reload uses the same absolute path resolution as startup
- Add "Reload Mappings" menu item if not already present

## Capabilities

### New Capabilities
- `mapping-loader`: Shared function to load and resolve mappings from the plugin directory's `mappings.toml`, used both at startup and on-demand reload

### Modified Capabilities
- *(none — no spec-level requirement changes)*

## Impact

- `xplane-uipc/src/lib.rs`: Startup calls shared loader; may extract `load_mappings_and_init` or refactor it
- `xplane-uipc/src/menu.rs`: "Reload Mappings" handler calls shared loader instead of inline reload code
- `xplane-uipc/src/plugin_state.rs`: `reload_mappings()` (if it exists) is replaced or refactored to delegate to shared loader
- `xplane-uipc/src/mapping.rs`: Shared `load_mappings` function already exists; may add a convenience wrapper for path construction
