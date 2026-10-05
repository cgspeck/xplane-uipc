## Why

A bug sweep of `xplane_uipc` found several places where the values FSUIPC clients see, or the values written back to X-Plane, are wrong:

- **Stale offsets.** Table entries are never removed. When a mapping is deleted on reload, its dataref goes away (for example after an aircraft change), or it stops evaluating, the last value is still served indefinitely.
- **Writes decoded by size, not type.** A client write is decoded from its byte count alone: 1/2/4 bytes as unsigned, 8 bytes as f64. So an `i16` write of `-1` reaches X-Plane as `65535`, an `f32` write arrives as its raw bit pattern, and an `i64`/`u64` write is reinterpreted as a double. Writes with an unsupported size are forwarded as `0.0`.
- **Truncating conversion.** Float→integer conversion truncates, so a scaled value like `1023.9999` becomes `1023` and intermittent off-by-one errors appear. Negative values in unsigned offsets clamp to 0 instead of wrapping, and NaN becomes 0. Writes to integer datarefs also truncate.
- **Read overrun.** IPC reads write the full width of the stored value whatever the request's `nBytes`, so reading 2 bytes from a 4-byte offset overwrites the next record header in the client's shared buffer.
- **Strings.** A string dataref that returns 0 bytes leaves the offset unset rather than empty. A string that fills its `size` exactly is served without a terminating NUL.

## What Changes

- `PluginState::update()` removes table entries for offsets that produced no value this cycle. A mapping whose `update_if_expr` is false keeps its previous value and stays active.
- Writes are decoded using the type of the offset's table entry: signed/unsigned 8–64-bit integers, f32, f64. A write whose `nBytes` doesn't match the entry's width is rejected and not forwarded.
- A single shared `Value::from_f64(f64, FsuipcType) -> Option<Value>` replaces the duplicate conversions in `xplane_uipc` and `uipc-debug`. It rounds to nearest, wraps out-of-range integers (two's complement), and returns `None` for non-finite input.
- Writes to integer datarefs (scalar and array) round to nearest.
- IPC reads copy at most `nBytes` (the low-order little-endian bytes of the value).
- String reads always return a NUL-terminated buffer of at most `size` bytes. A 0-byte read is served as an empty string.

## Non-goals

- Missing datarefs inside an expression keep evaluating as `0.0`. This is documented in `README-EXPR.md`, and the shipped lights mapping (0x0D0C) depends on it to mix generic and aircraft-specific datarefs. Only the misleading doc comment on `read_xplane` is corrected.
- Writable strings and writable expression mappings remain unsupported.

## Capabilities

### New Capabilities
- `offset-value-semantics`: how mapped values are converted, served, retained, expired and written back.

### Modified Capabilities
_None_

## Impact

- `ipc_host/src/value_table.rs`: `Value::from_f64`, `Value::to_le_bytes`, `Value::decode_le`, cycle tracking for removing stale entries. `ipc_host` gains a dependency on the portable `uipc-mapping` crate (for `FsuipcType`).
- `ipc_host/src/mapped_view.rs`: type-driven write decoding and `nBytes`-bounded reads.
- `xplane_uipc/src/plugin_state.rs`: uses the shared conversion, retain/expire semantics in `update()`, rounding integer writes, string NUL handling.
- `uipc-debug/src/tui.rs`: uses the shared conversion.
- Client-visible: values may differ by ±1 where truncation used to apply. Negative values in unsigned offsets now wrap instead of reading 0. Offsets without a current value now return the "not in table" path instead of a stale value.
