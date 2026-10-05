## Context

The flight loop (`PluginState::update`) evaluates every mapping and writes the result into the global `Table`. The IPC thread reads the table to answer client reads, and turns client writes into `WriteRequest { offset, value: f64, size }` that the flight loop applies through `ResolvedMapping::write_xplane`.

Today:
- `Table::clear_active_and_writable()` resets the active/writable bookkeeping but never removes `entries[]`. Reads go straight to `entries[]`, so anything inserted once is served forever.
- `mapped_view` decodes a write's payload from `nBytes` alone.
- `f64_to_value` uses `as` casts, which truncate and saturate. It is duplicated in `uipc-debug/src/tui.rs`.
- Reads use `write_unaligned::<T>` with the stored value's width, ignoring `nBytes`.

## Goals / Non-Goals

**Goals:**
- An offset is served only if its mapping produced (or deliberately retained) a value in the latest update cycle.
- Writes respect the offset's declared type.
- One tested conversion from f64 to `Value`.
- No IPC read writes outside its record's payload.

**Non-Goals:**
- Changing the missing-variable-as-0.0 behaviour of expressions.
- Writable expressions or strings.

## Decisions

### 1. Per-cycle expiry in `Table`
Add `Table::begin_update() -> Vec<u16>`, which returns the previous cycle's active offsets and clears the bookkeeping, and `Table::end_update(previous)`, which sets `entries[i] = None` for each previous offset not re-activated this cycle. Add `Table::keep(index)`, which re-activates an existing entry without changing its value (restoring its writable flag from the entry).

`update()` becomes: `prev = begin_update()`; for each mapping, insert the value, or `keep()` on retain; then `end_update(prev)`. All of this happens under the existing write lock, so readers never see a half-built cycle.

*Alternative considered:* clear the whole table each cycle and re-insert everything. Rejected because it touches 65536 slots at 20 Hz, and retained values would then need to live somewhere else.

### 2. Retain vs missing
`ResolvedMapping::read_xplane_value()` returns a small enum, `Reading::{Value(Value), Retain, Missing}`. `update_if_expr` evaluating to false yields `Retain`. A null handle, failed read or non-finite result yields `Missing`. The current "None means retain" behaviour only worked by accident (because entries were never removed), so this makes it explicit.

### 3. Conversion: `Value::from_f64(v, FsuipcType) -> Option<Value>`
Lives in `ipc_host::value_table`, so it is unit-testable on any host where `ipc_host` tests run, and shared with `uipc-debug`. `ipc_host` takes a path dependency on `uipc-mapping` (portable, no Windows deps) for `FsuipcType`.
- Non-finite input → `None`.
- Integers: `r = v.round()` (half away from zero).
  - Types ≤ 32 bits: `(r as i64) as T`. This wraps two's complement, matching how FSUIPC exposes raw memory: `-1` as `u16` is `0xFFFF`.
  - `i64`: `r as i64` (saturating).
  - `u64`: negative → `(r as i64) as u64`, else `r as u64`.
- `f32`/`f64`: plain casts.
- `String` → `None` (strings don't come from f64).

### 4. Type-driven write decoding
`Value::decode_le(&self, bytes) -> Option<f64>` decodes `bytes` using the stored entry's variant as the type, and requires `bytes.len()` to equal the variant's width. `mapped_view` looks up the entry (it already has the table) and forwards a write only when decoding succeeds. Otherwise it counts an error and logs a warning, with no write. Strings and bools aren't writable through this path.

*Alternative considered:* send raw bytes over the channel and decode in `xplane_uipc` using `FsuipcType`. Rejected because the entry variant already encodes the type, and keeping `WriteRequest` as f64 avoids touching the channel type.

### 5. Bounded reads
`Value::to_le_bytes(&self) -> Vec<u8>` for numeric variants. `mapped_view` copies `min(len, nBytes)` bytes (the low-order bytes, matching a little-endian memory read of a narrower width). The existing string path is unchanged: it already bounds and zero-fills.

### 6. Rounding integer writes
`ResolvedRef::write` uses `xplane_value.round() as i32` for `Int` and `IntArray` datarefs.

### 7. Strings
`read_bytes(size)`: truncate to `size - 1` bytes when no NUL fits, then append NUL, so the result is never longer than `size`. A 0-byte read returns `vec![0]` (an empty string) rather than `None`. A null handle is still `Missing`.

## Risks / Trade-offs

- **Clients relying on stale values.** For example, a value that was last seen before an aircraft change. Now they get "not in table" until the dataref is available. This is the intended behaviour.
- **±1 differences** where truncation used to apply. This is intended. FSUIPC clients expect correctly rounded values.
- **Wrapping vs saturating** out-of-range signed values (e.g. `40000` into `i16` → `-25536`). This is consistent with raw-memory semantics, and the mapping author controls the scale.
- **New `ipc_host` → `uipc-mapping` dependency.** It is small and portable, and `uipc-debug` already depends on both.
