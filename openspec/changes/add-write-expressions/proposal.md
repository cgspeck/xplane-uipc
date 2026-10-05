## Why

Client writes are only applied to single-dataref mappings, by undoing `scale`/`offset_add`. Expression mappings can be read but not written, and the plugin warns at load that writes to them "will have no effect". Several offsets clients commonly write can't be expressed as one dataref with a linear transform:

- **Time (`0x0238`–`0x023C`)**: the hour/minute/second datarefs are read-only. Setting the time means merging the changed field into `sim/time/zulu_time_sec`, keeping the fields that weren't written.
- **Lights (`0x0D0C`)**: one write sets several switch datarefs, one per bit.
- **Spoilers (`0x0BD0`)**: piecewise. 4800 means armed (`speedbrake_ratio = -0.5`), and 5620–16383 means deployed (0–1).
- **Battery (`0x3102`, `0x281C`)**: the read ORs the Zibo and generic battery datarefs, so a write has to set both.
- **SquawkBox transponder mode (`0x7B91`)**: the reverse of `transponder_mode < 2` depends on the current mode.

## What Changes

- Add an optional `write_exprs` table to expression mappings. Each key names an entry in the mapping's `datarefs`. Each value is an RPN expression that works out what to write to that dataref, using `$value` (the value the client wrote, in FSUIPC units) and the current values of the mapping's datarefs.
- On a client write, the plugin evaluates every write expression first, then writes the results. If any result is not a finite number, nothing is written.
- The loader validates `write_exprs`:
  - It needs an `expr` mapping with `writable = true`.
  - Every key must name a `datarefs` entry.
  - Every variable must be `$value` or a `datarefs` name.
  - A `datarefs` entry may not be called `value`.
- The load-time writability warning covers write targets: a target dataref that X-Plane reports as read-only gets a warning.
- Every writable mapping must have a way to be written. These become load errors:
  - **BREAKING** A writable expression mapping without `write_exprs`. Today it loads with a "writes will have no effect" warning and drops every client write without a word.
  - **BREAKING** A writable single-dataref mapping with `scale = 0`. Its read is the constant `offset_add`, which can't be reversed. Today the write quietly uses a scale of 1.
  - **BREAKING** A mapping with both `dataref` and `expr`. Today `expr` wins and `dataref`, `scale` and `offset_add` are ignored without a word.

  No mapping in the shipped `mappings.toml` hits any of these.
- Writes that don't happen are reported. A single-dataref write whose dataref is unavailable at runtime logs a warning, the same as an unavailable write-expression target. `write_offset` logs "Wrote value…" only when a dataref was actually written.
- Update `mappings.toml` to make the offsets above writable, and document `write_exprs` in the mapping file header and `README-EXPR.md`.

The expression language and `uipc-expr` don't change. The write side only needs the existing evaluator and a variable map.

## Capabilities

### New Capabilities
- `write-expressions`: Expression mappings that turn a client write into one or more dataref writes.

### Modified Capabilities

- `offset-value-semantics`: Writable mappings have to be reversible (no `scale = 0`, no expression mapping without `write_exprs`). Writes that don't reach a dataref are logged as such. The linear reverse itself is unchanged.
- `mapping-loader`: A mapping can't have both `dataref` and `expr`.

## Impact

- **`uipc-mapping`**: `RawMapping` gains `write_exprs`, and `MappingSource::Expr` gains the parsed write expressions. New validation rules and load errors, including the `dataref`+`expr`, `scale = 0` and missing-`write_exprs` checks.
- **`xplane_uipc/src/plugin_state.rs`**: `ResolvedSource::Expr` keeps the write expressions. `write_xplane` evaluates them, writes each target and reports whether anything was written. `ResolvedRef::write` reports an unavailable dataref instead of returning quietly. The load-time writability check covers write targets, and the "only single-dataref mappings can be written" warning goes away (it's now a load error).
- **Existing user `mappings.toml` files**: A file that hits one of the new load errors loses those mappings. The load error names the offset and the reason.
- **`ipc_host`**: No change. Writes are already decoded by the offset's type and sent to the flight loop as an `f64`.
- **`mappings.toml`**: Write expressions for `0x0238`–`0x023C`, `0x0D0C`, `0x0BD0`, `0x3102`, `0x281C` and `0x7B91`.
- **Docs**: `mappings.toml` header comments and `uipc-expr/README-EXPR.md`.
