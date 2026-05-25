## Context

The `Static` mapping variant is already defined in both `MappingSource` and `ResolvedSource` enums, the `RawMapping` struct already has the `static_value: Option<f64>` field, and the runtime evaluation path (`ResolvedMapping::new()`, `read_xplane()`) already handles it. The only gap is that `load_mappings()` in `mapping.rs` never checks `r.static_value` — it falls through to an error "must have either 'dataref' or 'expr'". Currently ~20 entries in `mappings.toml` use `expr = "1"` / `expr = "0"` with empty `datarefs = {}` as a workaround.

## Goals / Non-Goals

**Goals:**
- Add a `static_value` branch in `load_mappings()` to produce `MappingSource::Static` when `r.static_value` is `Some`
- Update validation error message to include `static_value` as a valid source
- Add unit tests for `load_mappings()` coverage across all three source types
- Optionally convert existing workaround entries in `mappings.toml` to use `static_value`

**Non-Goals:**
- No changes to the runtime evaluation path (already complete)
- No changes to serialization (mappings are never written back to TOML)
- No changes to `write_xplane()` (static values are read-only by nature)
- No new mapping type discovery or plugin system

## Decisions

1. **Priority order**: `expr` > `dataref` > `static_value` — If a mapping has both `expr` and `static_value`, `expr` wins. This preserves backward compatibility with existing entries.
2. **No validation warning for redundant fields**: A mapping with `static_value` plus `dataref` or `expr` silently uses the higher-priority source. This matches existing behavior where `expr + datarefs` ignores `dataref` if both are present.
3. **Test approach**: Add `#[cfg(test)]` module tests using inline TOML strings via `toml::from_str`, avoiding the need for test fixture files in the filesystem.

## Risks / Trade-offs

- [Low] **TOML file entries with `static_value` pick up a field they didn't have before** — Since `static_value: Option<f64>` already exists in `RawMapping` with no serde default, and no current TOML entries use `static_value`, this is a non-issue.
- [Low] **Converting existing workaround entries** carries minor risk if someone is relying on the expression evaluation behavior for what appears to be a constant. Verify each entry is truly a constant expression.
