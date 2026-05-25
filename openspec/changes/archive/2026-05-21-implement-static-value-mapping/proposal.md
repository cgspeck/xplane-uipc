## Why

The `Static` mapping variant is fully defined in the type system and runtime (enum variants, resolution, and evaluation all exist), but `load_mappings()` never checks for `static_value` in the TOML input. Currently, static values are achieved via a workaround using `expr = "1"` with `datarefs = {}`, which is semantically misleading and unnecessarily creates an `Expr` and empty `HashMap` for what is just a constant. Adding proper deserialization support completes the implementation with minimal code changes.

## What Changes

- Add a `static_value` branch in `load_mappings()` to create `MappingSource::Static` when `r.static_value` is `Some`
- Update the validation error message to acknowledge `static_value` as a valid source
- Add unit tests for `load_mappings()` covering all three source types (Simple, Expr, Static)
- (Optional) Convert existing `expr = "1"` / `expr = "0"` workaround mappings in `mappings.toml` to use `static_value` instead

## Capabilities

### New Capabilities
- `static-value-mapping`: Support for defining mapping entries with a fixed constant value via `static_value` in `mappings.toml`, as a proper alternative to the expression-based workaround

### Modified Capabilities

<!-- No existing specs to modify -->

## Impact

- **`xplane_uipc/src/mapping.rs`**: Add `static_value` branch in `load_mappings()` deserialization logic and update error message
- **`xplane_uipc/mappings.toml`**: Optionally convert ~20 existing workaround entries to use `static_value`
- **Tests**: Add unit tests for `load_mappings()` (currently no tests exist for this function)
