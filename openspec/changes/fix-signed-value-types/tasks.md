## Tasks

- [ ] 1. Add signed and float variants to Value enum (`ipc_host/src/value_table.rs`)
- [ ] 2. Fix creation site: `populate_table()` (`xplane_uipc/src/plugin_state.rs`)
- [ ] 3. Fix creation site: `update()` (`xplane_uipc/src/plugin_state.rs`)
- [ ] 4. Fix creation site: debug TUI (`uipc-debug/src/tui.rs`)
- [ ] 5. Add consumption arms in `process_mapped_view()` (`ipc_host/src/mapped_view.rs`)
- [ ] 6. Add round-trip tests for all FsuipcType variants covering positive, negative, and boundary values

### Task Details

### 1. Add signed and float variants to Value enum
**File:** `ipc_host/src/value_table.rs`

Add 5 new variants to the `Value` enum:
- `SignedInt8(i8)`
- `SignedInt16(i16)`
- `SignedInt32(i32)`
- `UnsignedInt64(u64)`
- `Float32(f32)`

### 2. Fix creation site: `populate_table()`
**File:** `xplane_uipc/src/plugin_state.rs` (lines 241-254)

Split the combined match arms so each FsuipcType maps to the correct Value variant:
- `I8` → `Value::SignedInt8(value as i8)`
- `U8` → `Value::UnsignedInt8(value as u8)`
- `I16` → `Value::SignedInt16(value as i16)`
- `U16` → `Value::UnsignedInt16(value as u16)`
- `I32` → `Value::SignedInt32(value as i32)`
- `U32` → `Value::UnsignedInteger32(value as u32)`
- `F32` → `Value::Float32(value as f32)`
- `I64` → `Value::Integer64(value as i64)`
- `U64` → `Value::UnsignedInt64(value as u64)`
- `F64` → `Value::Float64(value)`

### 3. Fix creation site: `update()`
**File:** `xplane_uipc/src/plugin_state.rs` (lines 277-290)

Same changes as task 2 — these two match blocks are identical.

### 4. Fix creation site: debug TUI
**File:** `uipc-debug/src/tui.rs` (lines 124-154)

Same split of match arms as tasks 2 and 3.

### 5. Add consumption arms in `process_mapped_view()`
**File:** `ipc_host/src/mapped_view.rs` (lines 294-318)

Add match arms for the new Value variants:
- `Value::SignedInt8(v)` → `write_unaligned(... as *mut i8, v.to_le())`
- `Value::SignedInt16(v)` → `write_unaligned(... as *mut i16, v.to_le())`
- `Value::SignedInt32(v)` → `write_unaligned(... as *mut i32, v.to_le())`
- `Value::UnsignedInt64(v)` → `write_unaligned(... as *mut u64, v.to_le())`
- `Value::Float32(v)` → `write_unaligned(... as *mut f32, *v)`

### 6. Add round-trip tests for all FsuipcType variants
**File:** `ipc_host/src/value_table.rs` (or a new test module)

Test each type with representative values. For each, create a Value, insert into the table, retrieve it, and verify the stored value matches.

**Test cases:**

| FsuipcType | Test values |
|---|---|
| `U8` | 0, 127, 255 |
| `I8` | 0, 127, -1, -128 |
| `U16` | 0, 32767, 65535 |
| `I16` | 0, 32767, -1, -10, -32768 |
| `U32` | 0, 2147483647, 4294967295 |
| `I32` | 0, 2147483647, -1, -2147483648 |
| `U64` | 0, u64::MAX |
| `I64` | 0, i64::MAX, -1, i64::MIN |
| `F32` | 0.0, 3.14, -273.15 |
| `F64` | 0.0, 3.14159265358979, -273.15 |

For the specific bug that triggered this: test that `i16` with value `-10.0` stores as `SignedInt16(-10)` and can be read back correctly.
