## 1. Remove `array_index` from TOML deserialization

- [x] 1.1 Remove `array_index` field and `default_array_index` fn from `RawMapping` struct in `mapping.rs`
- [x] 1.2 Verify serde rejects unknown `array_index` fields (added `deny_unknown_fields` to `RawMapping`)

## 2. Route simple mappings through `parse_dataref_with_index`

- [x] 2.1 In `load_mappings` simple branch, apply `parse_dataref_with_index` to `dr` before constructing `MappingSource::Simple`
- [x] 2.2 Verify existing tests still pass (`cargo test`)
- [x] 2.3 Verify the fix with the actual `mappings.toml` ENGN entry

## 3. Fix `mappings.toml`

- [x] 3.1 Remove the `array_index = 0` line from the ENGN_N1_ entry (the `[0]` in the dataref string is now parsed instead)
- [x] 3.2 Run `cargo build` to confirm no warnings or errors

## 4. Verify

- [x] 4.1 Run `cargo fmt`
- [x] 4.2 Run `cargo test`
- [x] 4.3 Run `cargo build`
- [x] 4.4 Run `cargo xtask dist` (per AGENTS.md)
