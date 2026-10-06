## Why

Client writes are only applied to single-dataref mappings, by undoing `scale`/`offset_add`. Expression mappings can be read but not written, and the plugin warns at load that writes to them "will have no effect". Several offsets clients commonly write can't be expressed as one dataref with a linear transform:

- **Time (`0x0238`–`0x023C`)**: the hour/minute/second datarefs are read-only. Setting the time means merging the changed field into `sim/time/zulu_time_sec`, keeping the fields that weren't written.
- **Lights (`0x0D0C`)**: one write sets several switch datarefs, one per bit.
- **Spoilers (`0x0BD0`)**: piecewise. 4800 means armed (`speedbrake_ratio = -0.5`), and 5620–16383 means deployed (0–1).
- **Battery (`0x3102`, `0x281C`)**: the read ORs the Zibo and generic battery datarefs, so a write has to set both.
- **SquawkBox transponder mode (`0x7B91`)**: the reverse of `transponder_mode < 2` depends on the current mode.

Testing in the sim showed that writing datarefs isn't always enough:

- Add-on aircraft such as the Zibo 738 override their switch datarefs every frame. A battery or avionics write takes effect for a split second, then reverts. These aircraft only respond to **commands**, and often only to toggle commands, so the plugin has to compare the requested state with the current one and run the command only when they differ.
- One client write often has to do several things, such as setting the default landing-light dataref **and** running the Zibo landing-light command.
- A Zulu hour or minute write reverted the same way. This is most likely X-Plane's "use system time" setting, which resets the clock to the system time.

## What Changes

- Add an optional `write_exprs` table to expression mappings. Each key names an entry in the mapping's `datarefs`. Each value is an RPN expression that works out what to write to that dataref, using `$value` (the value the client wrote, in FSUIPC units) and the current values of the mapping's datarefs.
- Add an optional `write_commands` table to expression mappings. Each key is an X-Plane command path. Each value is an RPN expression, with the same variables as `write_exprs`, that gives how many times to run the command: 0 means don't run it. Each command's expression can pick out its own part of a compound value, such as one bit of the lights offset, and compare it with that part's current state.
- On a client write, the plugin evaluates every write expression and command expression first, against one snapshot of the datarefs. It then writes the datarefs and runs the commands. If any result is not a finite number, nothing is written and no command runs.
- A missing dataref or command is skipped and the rest still go ahead. The first skip of each target logs an error, and later skips log at debug until mappings are reloaded.
- The loader validates `write_exprs` and `write_commands`:
  - Each needs an `expr` mapping with `writable = true`.
  - Every `write_exprs` key must name a `datarefs` entry. Every `write_commands` key must be a command path (it contains `/`).
  - Every variable must be `$value` or a `datarefs` name.
  - A `datarefs` entry may not be called `value`.
- The load-time writability warning covers write targets: a target dataref that X-Plane reports as read-only gets a warning.
- Every writable mapping must have a way to be written. These become load errors:
  - **BREAKING** A writable expression mapping with neither `write_exprs` nor `write_commands`. Today it loads with a "writes will have no effect" warning and drops every client write without a word.
  - **BREAKING** A writable single-dataref mapping with `scale = 0`. Its read is the constant `offset_add`, which can't be reversed. Today the write quietly uses a scale of 1.
  - **BREAKING** A mapping with both `dataref` and `expr`. Today `expr` wins and `dataref`, `scale` and `offset_add` are ignored without a word.

  No mapping in the shipped `mappings.toml` hits any of these.
- Writes that don't happen are reported. A single-dataref write whose dataref is unavailable at runtime logs an error, the same as an unavailable write-expression target. `write_offset` logs "Wrote value…" only when a dataref was actually written or a command was run.
- Update `mappings.toml` to make the offsets above writable:
  - The time writes also set `sim/time/use_system_time` to 0.
  - Avionics (`0x2E80`) becomes an expression mapping so it can run commands.
  - Zibo commands for the battery, avionics and landing lights go in as commented placeholders, because their paths can't be checked against `DataRefs.txt`.
- Document `write_exprs` and `write_commands` in the mapping file header and `README-EXPR.md`.

The expression language and `uipc-expr` don't change. The write side only needs the existing evaluator and a variable map.

Datarefs and commands are still found once, on the first flight-loop frame after an aircraft loads (and on "Reload Mappings"). A dataref or command an add-on registers later isn't picked up until the next load.

## Capabilities

### New Capabilities
- `write-expressions`: Expression mappings that turn a client write into dataref writes and X-Plane commands.

### Modified Capabilities

- `offset-value-semantics`: Writable mappings have to be reversible (no `scale = 0`, no expression mapping without `write_exprs` or `write_commands`). Writes that don't reach a dataref are logged as such. The linear reverse itself is unchanged.
- `mapping-loader`: A mapping can't have both `dataref` and `expr`.

## Impact

- **`uipc-mapping`**: `RawMapping` gains `write_exprs` and `write_commands`, and `MappingSource::Expr` gains both, parsed. New validation rules and load errors, including the `dataref`+`expr`, `scale = 0` and missing-`write_exprs` checks.
- **`xplane_uipc/src/plugin_state.rs`**: `ResolvedSource::Expr` keeps the write expressions and the found commands. `write_xplane` evaluates them, writes each dataref, runs each command and reports whether anything was done. `ResolvedRef::write` reports an unavailable dataref instead of returning quietly. The load-time writability check covers write targets, and the "only single-dataref mappings can be written" warning goes away (it's now a load error).
- **Existing user `mappings.toml` files**: A file that hits one of the new load errors loses those mappings. The load error names the offset and the reason.
- **`xplane_uipc/build.rs`**: The bindgen allowlist gains `XPLMFindCommand` and `XPLMCommandOnce`.
- **`ipc_host`**: No change. Writes are already decoded by the offset's type and sent to the flight loop as an `f64`.
- **`mappings.toml`**: Write expressions for `0x0238`–`0x023C`, `0x0D0C`, `0x0BD0`, `0x2E80`, `0x3102`, `0x281C` and `0x7B91`, plus Zibo command placeholders.
- **Docs**: `mappings.toml` header comments and `uipc-expr/README-EXPR.md`.
