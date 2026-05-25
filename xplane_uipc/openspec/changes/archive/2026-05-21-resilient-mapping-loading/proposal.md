## Why

Currently, if any single mapping in `mappings.toml` fails to parse or validate, the entire mapping file is rejected and no mappings are loaded. This means a typo or misconfiguration in one mapping disables all mappings, making it difficult to diagnose which mapping is problematic and leaving the plugin with no working mappings at all.

## What Changes

- `mapping::load_mappings()` will no longer fail on individual mapping errors. Instead, each mapping is processed independently and failures are collected.
- Individual mapping parse/validation errors are logged with `tracing::error!` including the offset and error details, then processing continues with the next mapping.
- The function returns successfully if at least one mapping was loaded, along with a summary of how many mappings succeeded and failed.
- Unrecoverable errors (file not found, TOML parse failure, zero mappings loaded) still return `Err`.
- Callers (`load_mappings_and_init`, reload flow) receive a result that includes partial-success information for logging.

## Capabilities

### New Capabilities
- `resilient-mapping-loading`: Per-mapping error handling during load, allowing partial success. Individual mapping failures are logged but do not prevent other mappings from loading. Unrecoverable errors (file not found, unparseable TOML, zero mappings) still fail the entire operation.

### Modified Capabilities
<!-- None -->

## Impact

- `src/mapping.rs`: `load_mappings()` logic changes to collect per-mapping errors instead of failing fast. Return type may change to include success/failure counts.
- `src/lib.rs`: `load_mappings_and_init()` updated to handle partial-success results and log summary.
- `src/menu.rs`: Reload flow updated to handle partial-success results.
- No breaking changes to external APIs or mapping file format.
