## Why

The `[[mapping]]` TOML syntax requires two separate fields to specify an array dataref element: `dataref = "path"` and `array_index = N`. This is inconsistent with the `datarefs` expression syntax which already supports the `path[N]` shorthand via `parse_dataref_with_index`, and it's error-prone — users naturally write `ENGN_N1_[0]` in the dataref string itself, which fails because `XPLMFindDataRef` receives the brackets as part of the lookup name.

## What Changes

- **Simple mappings**: `dataref = "path[N]"` SHALL be parsed to extract index `N`, strip brackets from the path passed to `XPLMFindDataRef`, and use `N` as the array offset in `XPLMGetDatavi`/`XPLMGetDatavf`
- **Remove `array_index` field**: The standalone `array_index` TOML field on simple mappings SHALL be removed — index is always specified via bracket notation in the dataref string
- **BREAKING**: Any config using `array_index` as a separate field will produce a TOML unknown-field error (serde will reject it). Users must migrate to `dataref = "path[N]"` syntax.

## Capabilities

### New Capabilities
- `dataref-index-notation`: Parse `[N]` bracket notation in simple mapping dataref strings and remove the standalone `array_index` field

### Modified Capabilities
- (none)

## Impact

- `mapping.rs`: ~5 lines changed in `load_mappings` to route simple path through `parse_dataref_with_index`; remove `array_index` field from `RawMapping` struct; remove `default_array_index` fn
- `mappings.toml`: Remove the one uncommented `array_index = 0` line (it's redundant with `[0]` already in the dataref string)
- No changes to `plugin_state.rs`, `ResolvedRef`, or `MappingSource::Simple` internal struct — the change is entirely in the deserialization layer
