## Context

A client write reaches the flight loop as `WriteRequest { offset, value: f64, size }`. The IPC thread has already decoded it by the offset's FSUIPC type (`offset-value-semantics`). `PluginState::write_offset` finds the writable mapping for the offset and calls `ResolvedMapping::write_xplane`, which today only handles `ResolvedSource::Simple`:

```rust
dr.write((fsuipc_value - offset_add) / scale)   // ints are rounded in ResolvedRef::write
```

For anything other than `Simple`, `write_xplane` does nothing, and `write_offset` still logs "Wrote value…". `ResolvedRef::write` also returns quietly when its handle is null. A write can therefore vanish with nothing in the log beyond a load-time warning. When `scale` is 0, `write_xplane` uses 1 instead.

Expression mappings (`ResolvedSource::Expr { refs, expr, update_if_expr }`) are read by building a `HashMap<String, f64>` from every ref, with unavailable datarefs read as `0.0`, and calling `Expr::eval`. `uipc-expr` is a dependency-free, side-effect-free RPN evaluator. `Expr::vars()` lists the variables an expression uses.

## Goals / Non-Goals

**Goals:**
- Let an expression mapping turn one client write into writes to one or more of its datarefs, and into X-Plane commands.
- Keep `uipc-expr` pure and unchanged.
- Catch mistakes at load time rather than when a client writes.
- Never apply a partial write because one expression produced garbage.

**Non-Goals:**
- Finding datarefs or commands after the aircraft has loaded. Resolution stays where it is: on the first flight-loop frame after an aircraft loads, and on "Reload Mappings". A dataref or command an add-on registers later is missing until the next load.
- Write expressions for single-dataref (`dataref = ...`) mappings. Those keep their linear `scale`/`offset_add` reverse. Converting one to an expression mapping is the way to get a custom write.
- String offsets. String mappings can't have `expr` today.
- Moving the aircraft. Writing position offsets needs `XPLMWorldToLocal` and `local_x/y/z`, which an expression can't do.
- `0x023E` (day of year). It's linear (`$DayZ 1 +`) and can become a single-dataref mapping with `offset_add = 1` without this change.

## Decisions

1. **One expression per target, as a TOML table (`write_exprs`).** The key is a name from the mapping's `datarefs`, and the value is the expression for what to write there:

   ```toml
   datarefs    = { Nav = "sim/cockpit2/switches/navigation_lights_on", Bcn = "sim/cockpit2/switches/beacon_on" }
   expr        = "$Nav 1 * $Bcn 2 * +"
   writable    = true
   write_exprs = { Nav = "$value 1 & 0 !=", Bcn = "$value 2 & 0 !=" }
   ```

   - *Alternative:* a store operator in the RPN (Forth-style `!Name`) together with `dup`/`drop`. Rejected: it would give the evaluator side effects and a callback into the plugin, make write order and partial failure part of the language, and require stack bookkeeping that's hard to read inside TOML strings.
   - *Naming:* `write_exprs` matches the existing `expr` and `update_if_expr`, and can't be confused with the boolean `writable`.

2. **Variables.** A write expression sees `$value`, the decoded value the client wrote in FSUIPC units (the same number a read would produce), plus every name in the mapping's `datarefs`, holding that dataref's current value. Unavailable datarefs read as `0.0`, the same as in read expressions.

3. **Evaluate everything, then write.** On a write, the plugin reads the current values once, evaluates every write expression against that same snapshot, and only writes if every result is finite (`f64::is_finite`). The evaluator already turns division by zero and square roots of negative numbers into `0.0`, but `^` can still overflow to infinity, and a NaN dataref value would carry through. Otherwise it logs a warning naming the mapping and writes nothing. Writes go out in sorted key order, so behaviour and logs are repeatable. Each result goes through the existing `ResolvedRef::write`, so array indices and integer rounding work as they do for single-dataref writes. A target whose dataref is unavailable at runtime is skipped with a warning; the other targets are still written.

4. **Strict load-time validation.** Read expressions quietly treat unknown variables as `0.0`. For writes, a typo would silently write zero to a real dataref, so these are load errors that drop the mapping:
   - `write_exprs` on a mapping without `expr`
   - `write_exprs` without `writable = true`
   - a key that isn't a `datarefs` name
   - a variable that's neither `value` nor a `datarefs` name
   - a `datarefs` entry named `value` when `write_exprs` is present
   - an expression that fails to parse
   - an empty `write_exprs` table

5. **Writability warnings.** The load-time `XPLMCanWriteDataRef` check that covers single-dataref mappings is extended to every write target of an expression mapping.

6. **A writable mapping must be reversible.** `writable = true` promises a client that its write does something. Two cases break that promise today, quietly. Both become load errors:
   - **Expression mapping without `write_exprs`.** It has no way to be written. Today it gets a warning and every write is dropped, while the client sees success. With `write_exprs` available there's always a fix, so the warning becomes an error. `write_exprs` without `writable` is already an error (decision 4), so the two now have to come together.
   - **Single-dataref mapping with `scale = 0`.** The read is `dataref * 0 + offset_add`, a constant, so no dataref value maps back to the client's number. Today `write_xplane` swaps in a scale of 1 and writes `value - offset_add`, which isn't the reverse of anything. A read-only `scale = 0` mapping stays legal (it behaves like a static value), though `static_value` is the clearer way to write it.
   - *Alternative:* keep warnings. Rejected: a warning in `uipc.log` that nobody reads, plus a client that thinks its write landed, is the failure this change is meant to remove. No shipped mapping hits either case.

7. **One source per mapping.** The loader picks `expr` first, then `dataref`, then the static values. A mapping with both `dataref` and `expr` loses its `dataref`, `scale` and `offset_add` without a word. That was only confusing for reads. Now that there are two write paths (linear for `dataref`, `write_exprs` for `expr`), it decides how a write behaves, so it becomes a load error. String mappings already reject `dataref` together with `static_value_str`. This decision doesn't add the other combinations (such as `expr` with `static_value`); they don't affect writes, and they can be tightened separately.

8. **Report writes that don't happen.** Each of these is logged:
   - `ResolvedRef::write` returns whether it wrote, instead of returning quietly on a null handle or an unexpected dataref type.
   - For a single-dataref mapping, `write_xplane` warns when its dataref is unavailable. That matches what decision 3 does for write-expression targets.
   - `write_xplane` returns whether any dataref was written. `write_offset` logs "Wrote value…" at debug only in that case. When nothing was written, the warning has already been logged.

   Clients can write the same offset every frame, so a repeated warning could flood the log. Each mapping warns about an unavailable dataref once per load. Later writes log at debug. The flag goes back to false on "Reload Mappings", because resolving builds fresh `ResolvedMapping`s.

9. **Write commands (`write_commands`).** Sim testing showed that add-ons such as the Zibo 738 override their switch datarefs every frame. Writing those datarefs works for a split second, then reverts. These aircraft respond only to commands, and often only to toggles, so a write has to compare the requested state with the current one. A second table on expression mappings handles this:

   ```toml
   datarefs       = { Land = "sim/cockpit2/switches/landing_lights_on", ZLand = "laminar/B738/..." }
   writable       = true
   write_exprs    = { Land = "$value 4 & 0 !=" }
   write_commands = { "laminar/B738/.../toggle" = "$value 4 & 0 != $ZLand 0 != !=" }
   ```

   - **The key is the command path**, and the value is an expression with the same variables as `write_exprs` (decision 2). Each expression picks out its own part of a compound value. In the example, it masks the landing-light bit, compares it with the Zibo state, and gives 1 (toggle once) only when they differ. A pair of separate on and off commands gets one expression each.
   - **The result is a run count.** It is rounded and clamped to 0..=10: 0 means don't run, and n means run `XPLMCommandOnce` n times, which steps a multi-position switch. The cap stops a bad expression from firing a command thousands of times.
   - **One snapshot, datarefs first.** Command expressions are evaluated together with the write expressions (decision 3). A non-finite result in either table means nothing is written and no command runs. Datarefs are written first, then commands run in sorted path order. That way a command handler that reads a dataref sees the new default state.
   - **Lookup.** Commands are found with `XPLMFindCommand` when the mapping is resolved, alongside its datarefs, with a load-time warning for each one that isn't found.
   - **Validation** matches `write_exprs`: it needs `expr` and `writable = true`, the table can't be empty, each key must contain `/`, each expression must parse, and variables must be `value` or `datarefs` names. A writable expression mapping needs `write_exprs`, `write_commands` or both.
   - *Alternative:* a named `commands` table mirroring `datarefs`. Rejected: commands have no index or value, so an alias adds a level of indirection for nothing.

10. **Missing targets: skip, error once per target.** This replaces the per-mapping warning in decision 8. A dataref or command that isn't available when a client writes is skipped, and the other targets still go ahead. The first skip of each target since mappings were loaded logs an error naming the offset and the dataref or command. Later skips of the same target log at debug. Availability only changes when mappings are re-resolved, which builds fresh `ResolvedMapping`s, so "once per load" is the same as "on change".

## Mapping updates

These were checked with `expr-calculator`; writable targets were checked against X-Plane 12's `DataRefs.txt`.

| Offset | Targets | Write expression(s) |
|---|---|---|
| `0x023B` Zulu hour | `Z = sim/time/zulu_time_sec` | `Z = "$value 3600 * $Z 3600 % +"` |
| `0x023C` Zulu minute | `Z` | `Z = "$Z $Z 3600 % - $value 60 * + $Z 60 % +"` |
| `0x023A` second | `Z` | `Z = "$Z $Z 60 % - $value +"` |
| `0x0238` local hour | `Z`, reading `LH = clock_timer/local_time_hours` | `Z = "$Z $value $LH - 3600 * + 86400 + 86400 %"` |
| `0x0239` local minute | `Z`, reading `LM = clock_timer/local_time_minutes` | `Z = "$Z $value $LM - 60 * + 86400 + 86400 %"` |
| `0x0D0C` lights | nav, beacon, landing, taxi, strobe switches | `Nav = "$value 1 & 0 !="`, `Bcn = "$value 2 & 0 !="`, `Land = "$value 4 & 0 !="`, `Taxi = "$value 8 & 0 !="`, `Strb = "$value 16 & 0 !="` |
| `0x0BD0` spoilers | `SB = speedbrake_ratio` | `SB = "$value 4800 == -0.5 $value 5620 - 10763 / 0 max ?"` |
| `0x3102`, `0x281C` battery | `Zibo738`, `Generic` | `Zibo738 = "$value"`, `Generic = "$value"` |
| `0x2E80` avionics | `Av = avionics_power_on` | `Av = "$value 0 !="`. Converted from a single-dataref mapping so it can carry commands |
| `0x7B91` transponder | `T_Mode = transponder_mode` | `T_Mode = "$value 0 != 1 $T_Mode 2 < 2 $T_Mode ? ?"` (1 → standby; 0 → on, keeping an existing on/alt mode) |

The time offsets keep reading from the `clock_timer` datarefs, so their read expressions are just the matching variable. Only the writes go through `zulu_time_sec`. Local time is set by shifting Zulu time, because `sim/time/local_time_sec` is read-only.

Every time write also sets `UseSys = sim/time/use_system_time` to `0`. A Zulu hour or minute write was seen to revert within a moment. When "use system time" is on, X-Plane keeps resetting the clock to the system time, and that is the likely cause. This is still to be confirmed in the sim.

The Zibo battery, avionics and landing-light commands, and the Zibo state datarefs they compare against, go into `mappings.toml` as commented placeholders. `DataRefs.txt` lists neither `laminar/B738` datarefs nor commands, so their paths have to be confirmed in the sim with DataRefTool before they're enabled.

## Risks / Trade-offs

- [Medium] **Add-on datarefs may ignore writes.** A writable dataref that an add-on overrides every frame looks like a write that didn't stick. `write_commands` is the fix, but it needs each add-on's command and state-dataref paths, which only the sim can confirm.
- [Medium] **Toggle commands depend on the state dataref.** A toggle runs when the requested state differs from the state dataref. If that dataref lags the switch by a frame, two quick writes could toggle twice. Clients write switch offsets on user actions, not every frame, so this is unlikely.
- [Medium] **New load errors break some user mapping files.** A user `mappings.toml` with a writable expression mapping that has neither `write_exprs` nor `write_commands`, a writable `scale = 0` mapping, or `dataref` together with `expr` loses those mappings after upgrading. The shipped file is clean. The load error names the offset and says what to change, and the release notes call it out. The mapping was already doing nothing useful on write (or, for `dataref` + `expr`, not doing what it says).
- [Low] **Several targets aren't one atomic step.** All values are computed before writing, but X-Plane sees them as separate writes within one flight loop callback. Nothing renders in between, so a client can't see a half-applied state.
- [Low] **Rounding in the time maths.** Seconds writes drop the fractional second. That's acceptable for a time-of-day setting.

## Open Questions

- Should a write expression be able to return "no change" for a target (for example, a NaN sentinel that skips just that target)? It isn't needed for the offsets above. Leave it out until a mapping needs it.
