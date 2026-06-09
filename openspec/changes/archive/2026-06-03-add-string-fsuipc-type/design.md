## Context

The mapping system (`uipc-mapping`) defines `FsuipcType` with 10 numeric variants (`i8`–`f64`), each with a fixed size. The runtime pipeline in `plugin_state.rs` reads datarefs as `f64` via `XPLMGetDatai`/`XPLMGetDataf`/`XPLMGetDatad`, converts to `Value` via `f64_to_value()`, and writes the result as raw bytes into the FSUIPC shared memory via `process_mapped_view`.

Strings break this pipeline at every level:
- `XPLMGetDatab` returns raw bytes, not `f64`
- A string occupies multiple bytes at its offset (variable length)
- The value table needs a byte-slice variant, not a scalar
- The write path must copy N bytes, not a single typed value

The .NET test client already has `OffsetType.String` and `StringOffsetHandle`, so the consumer side is ready — only the plugin and mapping layers need the new type.

## Goals / Non-Goals

**Goals:**
- Add `FsuipcType::String` for null-terminated C-strings
- Config syntax: `fsuipc_type = "string"` with required `size` field and `static_value_str` or `dataref`
- Carry actual byte size through `DatarefMapping` so downstream code doesn't branch on type
- Add `Value::String(Vec<u8>)` to the value table
- Read string datarefs via `XPLMGetDatab` with null-termination enforcement
- Write string values into FSUIPC payload buffer with zero-fill padding

**Non-Goals:**
- Writable strings (`XPLMSetDatab`) — no use case yet
- `Expr` source for strings — expr evaluates to `f64`, not applicable
- `Bytes` FsuipcType — .NET client has it but no plugin use case yet
- Dynamic/runtime string sizing — size is fixed at config time

## Decisions

1. **Size lives in `DatarefMapping`, not `FsuipcType`** — `FsuipcType::size()` returns `1` for String (placeholder). The actual size comes from `RawMapping.size` config field and is stored as `DatarefMapping.size: usize`. This keeps the enum `Copy`-able and avoids embedding config values in the type system.

2. **Separate `static_value_str` field** — Rather than making `static_value` polymorphic (serde untagged), use a distinct `static_value_str: Option<String>` field. This is simpler to deserialize, clearer in TOML, and avoids any risk of breaking existing numeric `static_value` entries.

3. **String type rejects `expr` and `static_value` (f64)** — Validation enforces that `fsuipc_type = "string"` uses only `dataref` or `static_value_str`. Mixing string type with numeric sources is a config error.

4. **Unified `read_xplane_value()` method** — Instead of keeping the two-step `read_xplane()` → `f64_to_value()` and adding a parallel string path, introduce `read_xplane_value() -> Option<Value>` that handles both. This simplifies `populate_table` and `update` to a single call site.

5. **Null termination is enforced, not assumed** — `read_bytes()` scans for `\0` and appends one if missing. This handles datarefs that may or may not include the terminator, and ensures the FSUIPC offset always contains a valid C-string.

6. **Zero-fill padding** — When writing a string to the payload buffer, any bytes beyond the actual string length are zeroed. This prevents leaking adjacent memory and matches FSUIPC convention for fixed-size string offsets.

## Risks / Trade-offs

- [Low] **`Value::String` variant adds memory overhead** — Each string entry allocates a `Vec<u8>`. For typical use (a few string offsets, 20–40 bytes each), this is negligible.

- [Low] **`FsuipcType::size()` returning 1 for String is a lie** — Any code calling `ty.size()` on a String type gets 1, not the real size. Mitigated by the fact that `DatarefMapping.size` is the authoritative source and all downstream code uses that. Could return `usize::MAX` or panic instead, but returning 1 is safer for bounds checks.

- [Low] **Backward compatibility** — No existing TOML entries use `fsuipc_type = "string"`, so no migration risk. The `size` and `static_value_str` fields are `Option` with no serde default, so existing entries are unaffected.
