## Why

FSUIPC offsets contain null-terminated C-strings (e.g., airport identifiers, aircraft tail numbers, flight plan names). The current mapping system only supports numeric types (`u8`–`f64`), so string datarefs cannot be mapped. Adding a `string` type closes this gap and enables the .NET test client (which already has `OffsetType.String`) to read string offsets from the plugin.

## What Changes

### 1. `FsuipcType::String` variant (`uipc-mapping/src/types.rs`)

Add `String` to the enum. The `size()` method returns `0` for `String` — actual size comes from config. The `FromStr` impl gains `"string"`.

### 2. `RawMapping` gains `size` and `static_value_str` (`uipc-mapping/src/mapping.rs`)

```toml
[[mapping]]
offset      = 0x3160
fsuipc_type = "string"
size        = 24
static_value_str = "hello"
```

- `size: Option<usize>` — required when `fsuipc_type = "string"`, errors if missing
- `static_value_str: Option<String>` — static string value (alternative to `dataref`)
- Validation: if `fsuipc_type = "string"`, exactly one of `dataref` or `static_value_str` must be present; `expr` and `static_value` (f64) are rejected

### 3. `DatarefMapping` gains `size` field (`uipc-mapping/src/mapping.rs`)

Add `size: usize` to `DatarefMapping`. For numeric types this is `ty.size()`, for strings it comes from `RawMapping.size`. Downstream code reads `mapping.size` without branching.

### 4. `MappingSource::StaticStr` variant (`uipc-mapping/src/mapping.rs`)

```rust
enum MappingSource {
    Simple { dataref_path, array_index, scale, offset_add },
    Static { static_value: f64 },
    StaticStr { static_str: String },  // NEW
    Expr { datarefs, expr },
}
```

### 5. `Value::String(Vec<u8>)` variant (`ipc_host/src/value_table.rs`)

Add a new variant to the `Value` enum. The `Vec<u8>` contains the raw bytes including the null terminator.

### 6. `process_mapped_view` handles `Value::String` (`ipc_host/src/mapped_view.rs`)

In the read-response path, copy bytes into the payload buffer. Zero-fill any remaining bytes beyond the string length to avoid leaking adjacent memory.

```rust
Value::String(bytes) => {
    let len = bytes.len().min(record.n_bytes as usize);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), record.payload_ptr, len);
    for i in len..record.n_bytes as usize {
        *record.payload_ptr.add(i) = 0;
    }
}
```

### 7. `ResolvedRef::read_bytes()` (`xplane_uipc/src/plugin_state.rs`)

New method using `XPLMGetDatab`:

```rust
pub fn read_bytes(&self, max_len: usize) -> Option<Vec<u8>> {
    if self.handle.is_null() { return None; }
    let mut buf = vec![0u8; max_len];
    let bytes_read = unsafe {
        XPLMGetDatab(self.handle, buf.as_mut_ptr() as *mut _, 0, max_len as i32)
    };
    if bytes_read == 0 { return None; }
    buf.truncate(bytes_read as usize);
    // Enforce null termination
    if buf.last() != Some(&0) { buf.push(0); }
    Some(buf)
}
```

### 8. `ResolvedMapping::read_xplane_value()` (`xplane_uipc/src/plugin_state.rs`)

New method that returns `Option<Value>` directly, replacing the two-step `read_xplane()` → `f64_to_value()` for both strings and numerics:

```rust
pub fn read_xplane_value(&self) -> Option<Value> {
    match self.fsuipc_type {
        FsuipcType::String => {
            let bytes = match &self.source {
                ResolvedSource::Simple { dr, .. } => dr.read_bytes(self.size)?,
                ResolvedSource::StaticStr { static_str } => {
                    let mut b = static_str.as_bytes().to_vec();
                    b.push(0);
                    b
                }
                _ => return None,
            };
            Some(Value::String(bytes))
        }
        _ => self.read_xplane().map(|v| f64_to_value(v, self.fsuipc_type)),
    }
}
```

`populate_table` and `update` call `read_xplane_value()` instead of the current two-step.

## Scope

### In scope

- `FsuipcType::String` variant and `FromStr`/`size()` updates
- `RawMapping`: `size` and `static_value_str` fields, validation
- `DatarefMapping::size` field
- `MappingSource::StaticStr` variant
- `Value::String` variant
- `process_mapped_view` string write path
- `ResolvedRef::read_bytes()` using `XPLMGetDatab`
- `ResolvedMapping::read_xplane_value()` unified read path
- `populate_table` / `update` using new read path
- Tests for config parsing (string type, missing size, static_value_str, dataref)
- Tests for `read_bytes` null-termination logic
- Tests for `Value::String` in `process_mapped_view`

### Out of scope

- Writable strings (`XPLMSetDatab`)
- `Expr` source for strings
- `Bytes` FsuipcType (the .NET client has it, but no use case yet)
- Dynamic/runtime string sizing

## Example

```toml
# Static string
[[mapping]]
offset      = 0x3160
fsuipc_type = "string"
size        = 24
static_value_str = "hello"

# Dataref string
[[mapping]]
offset      = 0x3180
fsuipc_type = "string"
size        = 40
dataref     = "sim/flightmodel/position/theta"
```

## Capabilities

### New Capabilities

- Map string datarefs to FSUIPC offsets
- Use static string values in offsets

### Modified Capabilities

- Mapping loader now validates string-specific fields
- Value table supports string entries
- Shared memory write path handles multi-byte string payloads
