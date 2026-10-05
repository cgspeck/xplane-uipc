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
- The load-time writability warning covers write targets: a target dataref that X-Plane reports as read-only gets a warning. A writable expression mapping without `write_exprs` keeps its "writes will have no effect" warning.
- Update `mappings.toml` to make the offsets above writable, and document `write_exprs` in the mapping file header and `README-EXPR.md`.

The expression language and `uipc-expr` don't change. The write side only needs the existing evaluator and a variable map.

## Capabilities

### New Capabilities
- `write-expressions`: Expression mappings that turn a client write into one or more dataref writes.

### Modified Capabilities

<!-- None. Single-dataref writes (offset-value-semantics) are unchanged. -->

## Impact

- **`uipc-mapping`**: `RawMapping` gains `write_exprs`, and `MappingSource::Expr` gains the parsed write expressions. New validation rules and load errors.
- **`xplane_uipc/src/plugin_state.rs`**: `ResolvedSource::Expr` keeps the write expressions. `write_xplane` evaluates them and writes each target. The load-time writability check covers write targets.
- **`ipc_host`**: No change. Writes are already decoded by the offset's type and sent to the flight loop as an `f64`.
- **`mappings.toml`**: Write expressions for `0x0238`–`0x023C`, `0x0D0C`, `0x0BD0`, `0x3102`, `0x281C` and `0x7B91`.
- **Docs**: `mappings.toml` header comments and `uipc-expr/README-EXPR.md`.
