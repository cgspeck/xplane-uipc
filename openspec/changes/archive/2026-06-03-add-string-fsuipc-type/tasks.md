## Tasks

- [x] 1. Add `FsuipcType::String` variant (`uipc-mapping/src/types.rs`) — Add `String` to enum, update `size()` to return `1` (placeholder) for String, add `"string"` arm to `FromStr`
- [x] 2. Add `DatarefMapping::size` field (`uipc-mapping/src/mapping.rs`) — Add `pub size: usize` to `DatarefMapping`, set it to `ty.size()` for numerics in `load_mappings`
- [x] 3. Add `RawMapping` fields for strings (`uipc-mapping/src/mapping.rs`) — Add `size: Option<usize>` and `static_value_str: Option<String>` to `RawMapping`, update `DatarefMapping.size` to use config size when `fsuipc_type == String`
- [x] 4. Add `MappingSource::StaticStr` variant (`uipc-mapping/src/mapping.rs`) — Add `StaticStr { static_str: String }` variant, wire it in `load_mappings` from `static_value_str`
- [x] 5. Validate string mapping constraints (`uipc-mapping/src/mapping.rs`) — Error if `fsuipc_type = "string"` and `size` is missing; error if neither `dataref` nor `static_value_str` provided; error if `expr` or `static_value` (f64) provided with string type
- [x] 6. Add `Value::String(Vec<u8>)` variant (`ipc_host/src/value_table.rs`) — Add variant to enum
- [x] 7. Handle `Value::String` in `process_mapped_view` (`ipc_host/src/mapped_view.rs`) — Copy bytes into payload buffer, zero-fill remaining space
- [x] 8. Add `ResolvedRef::read_bytes()` (`xplane_uipc/src/plugin_state.rs`) — Use `XPLMGetDatab` to read raw bytes, enforce null termination
- [x] 9. Add `ResolvedMapping::read_xplane_value()` (`xplane_uipc/src/plugin_state.rs`) — Unified method returning `Option<Value>`, handle String via `read_bytes` and `StaticStr`, existing types via `read_xplane` + `f64_to_value`
- [x] 10. Update `populate_table` and `update` to use `read_xplane_value()` (`xplane_uipc/src/plugin_state.rs`) — Replace two-step read+f64_to_value with single `read_xplane_value()` call
- [x] 11. Add config parsing tests (`uipc-mapping/src/mapping.rs`) — Test: string with static_value_str, string with dataref, string missing size errors, string with expr errors, string with static_value (f64) errors
- [x] 12. Add `read_bytes` and `read_xplane_value` tests (`xplane_uipc/src/plugin_state.rs`) — Test null-termination enforcement, string Value round-trip through mapped_view
- [x] 13. Run formatter, tests, build, and dist (`cargo fmt`, `cargo test`, `cargo build`, `cargo xtask dist`)

### Task Details

### 1. Add `FsuipcType::String` variant
**File:** `uipc-mapping/src/types.rs`

- Add `String` to the `FsuipcType` enum
- In `size()`, add `Self::String => 1` (placeholder; actual size comes from config)
- In `FromStr`, add `"string" => Ok(Self::String)`

### 2. Add `DatarefMapping::size` field
**File:** `uipc-mapping/src/mapping.rs`

- Add `pub size: usize` to `DatarefMapping` struct
- In `load_mappings`, when building `DatarefMapping`, set `size: r.fsuipc_type.size()` for non-string types

### 3. Add `RawMapping` fields for strings
**File:** `uipc-mapping/src/mapping.rs`

- Add `size: Option<usize>` to `RawMapping`
- Add `static_value_str: Option<String>` to `RawMapping`
- When `fsuipc_type == FsuipcType::String`, use `r.size.unwrap_or(0)` as the mapping size instead of `r.fsuipc_type.size()`

### 4. Add `MappingSource::StaticStr` variant
**File:** `uipc-mapping/src/mapping.rs`

- Add `StaticStr { static_str: String }` to `MappingSource` enum
- In `load_mappings`, when `static_value_str` is `Some(s)`, produce `MappingSource::StaticStr { static_str: s }`

### 5. Validate string mapping constraints
**File:** `uipc-mapping/src/mapping.rs`

In `load_mappings`, add validation for string type:
- If `fsuipc_type == String` and `size` is `None` → error "string type requires 'size' field"
- If `fsuipc_type == String` and `expr` is `Some` → error "string type does not support 'expr'"
- If `fsuipc_type == String` and `static_value` is `Some` → error "string type uses 'static_value_str' instead of 'static_value'"
- If `fsuipc_type == String` and neither `dataref` nor `static_value_str` → error
- If `fsuipc_type == String` and both `dataref` and `static_value_str` → error (mutually exclusive)

### 6. Add `Value::String` variant
**File:** `ipc_host/src/value_table.rs`

- Add `String(Vec<u8>)` to the `Value` enum

### 7. Handle `Value::String` in `process_mapped_view`
**File:** `ipc_host/src/mapped_view.rs`

- In the read-response match, add a `Value::String(bytes)` arm:
  - Copy `min(bytes.len(), n_bytes)` bytes to `record.payload_ptr`
  - Zero-fill remaining bytes up to `n_bytes`
- Add test: string value written correctly to payload buffer

### 8. Add `ResolvedRef::read_bytes()`
**File:** `xplane_uipc/src/plugin_state.rs`

- New method: `pub fn read_bytes(&self, max_len: usize) -> Option<Vec<u8>>`
- Allocate `vec![0u8; max_len]`, call `XPLMGetDatab`, truncate to bytes read
- Enforce null termination: if last byte is not `\0`, push `\0`
- Return `None` if handle is null or bytes_read is 0

### 9. Add `ResolvedMapping::read_xplane_value()`
**File:** `xplane_uipc/src/plugin_state.rs`

- New method: `pub fn read_xplane_value(&self) -> Option<Value>`
- If `fsuipc_type == String`: dispatch to `read_bytes` (Simple) or build from `static_str` (StaticStr)
- Otherwise: delegate to `read_xplane()` + `f64_to_value()`

### 10. Update `populate_table` and `update`
**File:** `xplane_uipc/src/plugin_state.rs`

- Replace `m.read_xplane()` + `f64_to_value(value, m.fsuipc_type)` with `m.read_xplane_value()`
- Both `populate_table` and `update` use the same pattern: `if let Some(value) = m.read_xplane_value() { ... }`

### 11. Add config parsing tests
**File:** `uipc-mapping/src/mapping.rs`

- `string_static_value_str` — valid mapping with `static_value_str`, verify `MappingSource::StaticStr`
- `string_dataref` — valid mapping with `dataref`, verify `MappingSource::Simple`
- `string_missing_size` — `fsuipc_type = "string"` without `size`, expect error
- `string_with_expr` — `fsuipc_type = "string"` with `expr`, expect error
- `string_with_static_value_f64` — `fsuipc_type = "string"` with `static_value` (f64), expect error
- `string_with_both_dataref_and_static_value_str` — expect error

### 12. Add read/write tests
**File:** `xplane_uipc/src/plugin_state.rs` or integration test

- Test `read_bytes` null-termination: dataref returns bytes without `\0`, verify `\0` appended
- Test `Value::String` round-trip through `process_mapped_view`: insert string entry, process read request, verify bytes in payload

### 13. Run verification

```bash
cargo fmt
cargo test
cargo build
cargo xtask dist
```
