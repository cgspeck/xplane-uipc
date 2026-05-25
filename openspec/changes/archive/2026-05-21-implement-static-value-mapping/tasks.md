## 1. Core Implementation

- [x] 1.1 Add `static_value` branch in `load_mappings()` — insert `else if let Some(sv) = r.static_value` before the final `else` to produce `MappingSource::Static { static_value: sv }`
- [x] 1.2 Update error message in the final `else` branch to mention `static_value` alongside `dataref` and `expr`
- [x] 1.3 Verify priority ordering: `expr` > `dataref` > `static_value` (static_value is lowest priority, checked last)

## 2. Testing

- [x] 2.1 Add `#[cfg(test)]` module tests in `mapping.rs` for `load_mappings()` with inline TOML strings, covering:
  - `static_value` alone produces `MappingSource::Static`
  - `static_value` with zero/negative values
  - Priority: `expr + static_value` → `MappingSource::Expr`
  - Priority: `dataref + static_value` → `MappingSource::Simple`
  - No source fields → descriptive error mentioning all three options
- [x] 2.2 Verify tests pass with `cargo test -p xplane_uipc`

## 3. Optional: TOML Cleanup

- [x] 3.1 Convert ~28 existing workaround entries (`datarefs = {}` + simple `expr` like `"1"`, `"0x50000008"`) in `mappings.toml` to use `static_value` instead
- [x] 3.2 Verify converted entries produce identical output values by running the test suite
