## Why

The `Value` enum in the value table only has unsigned integer variants (`UnsignedInt8`, `UnsignedInt16`, `UnsignedInteger32`) plus `Integer64` and `Float64`. When a mapping declares `fsuipc_type = "i16"` (or i8, i32), the f64 value from X-Plane is cast to the unsigned type (`value as u16`). In Rust, casting a negative float to an unsigned integer saturates to 0. This means **any signed mapping that produces a negative value silently returns 0 to FSUIPC clients**.

The bug has been hidden because existing signed mappings happen to always produce non-negative values at runtime (e.g., VVI uses `-60 *` which negates an already-negative descent rate). The new UTC offset mapping (0x0246, i16) exposed it — negative UTC offsets (local ahead of zulu) return 0 instead of the correct negative value.

Additionally, `F32` is stored as `UnsignedInteger32(value as u32)`, which destroys the float representation entirely.

## What Changes

### Value enum expansion

Add signed integer variants and a float32 variant to the `Value` enum:

| Current | Problem | Fix |
|---|---|---|
| `I8 → UnsignedInt8(value as u8)` | Negative values saturate to 0 | Add `SignedInt8(i8)` |
| `I16 → UnsignedInt16(value as u16)` | Negative values saturate to 0 | Add `SignedInt16(i16)` |
| `I32 → UnsignedInteger32(value as u32)` | Negative values saturate to 0 | Add `SignedInt32(i32)` |
| `F32 → UnsignedInteger32(value as u32)` | Float bits destroyed | Add `Float32(f32)` |
| `U64 → Integer64(value as i64)` | Large u64 values misinterpreted | Add `UnsignedInt64(u64)` |

### Sites to update

Three creation sites all have identical match arms that need fixing:

1. **`plugin_state.rs` — `populate_table()`** (lines 241-254)
2. **`plugin_state.rs` — `update()`** (lines 277-290)
3. **`uipc-debug/src/tui.rs`** (lines 124-154)

One consumption site:

4. **`mapped_view.rs` — `process_mapped_view()`** (lines 294-318) — add match arms for new variants

### Correct mapping

After the fix:

| FsuipcType | Value variant | Cast |
|---|---|---|
| `U8` | `UnsignedInt8(value as u8)` | unchanged |
| `I8` | `SignedInt8(value as i8)` | new |
| `U16` | `UnsignedInt16(value as u16)` | unchanged |
| `I16` | `SignedInt16(value as i16)` | new |
| `U32` | `UnsignedInteger32(value as u32)` | unchanged |
| `I32` | `SignedInt32(value as i32)` | new |
| `U64` | `UnsignedInt64(value as u64)` | new |
| `I64` | `Integer64(value as i64)` | unchanged |
| `F32` | `Float32(value as f32)` | new |
| `F64` | `Float64(value)` | unchanged |

## Test plan

Add a comprehensive test that round-trips every FsuipcType through the value table and verifies correctness, including:

- Positive values for all types
- Negative values for all signed types (i8, i16, i32, i64)
- Boundary values (i16::MIN, i16::MAX, u16::MAX, etc.)
- Float precision (f32 and f64)
- The specific failing case: i16 with value -10.0 (the UTC offset bug)

## Scope

### In scope
- Value enum changes in `ipc_host/src/value_table.rs`
- All three creation sites (plugin_state.rs x2, tui.rs)
- The consumption site (mapped_view.rs)
- Comprehensive round-trip tests

### Out of scope
- Renaming existing variants for consistency (e.g., `UnsignedInteger32` vs `UnsignedInt8`) — cosmetic, can do separately
- Changes to `fsuipc_offsets.rs` — already handles signedness correctly via `write_value`/`read_value`
