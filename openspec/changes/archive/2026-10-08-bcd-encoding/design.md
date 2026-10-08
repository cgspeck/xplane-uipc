# Design

## Context

Expression mappings already have everything needed apart from the conversion itself. `expr` computes the served value, which `Value::from_f64` rounds and wraps to the type's width. `write_exprs` gets the client's value as `$value`. A non-finite result means the offset isn't served (on read) or nothing is written (on write). The plugin and `uipc-debug` both evaluate through `uipc_expr::Expr`, so a new operator is available everywhere at once.

The relevant X-Plane datarefs are all writable `int`s:

| Offset | FSUIPC meaning | Dataref | Dataref example |
|---|---|---|---|
| `0x0354` | squawk, 4 BCD digits | `sim/cockpit2/radios/actuators/transponder_code` | `2770` |
| `0x034E` / `0x3118` | COM1 / COM2 active, leading 1 dropped | `com1_frequency_hz` / `com2_frequency_hz` | `12345` |
| `0x311A` / `0x311C` | COM1 / COM2 standby | `com1_standby_frequency_hz` / `com2_...` | `12345` |
| `0x0350` / `0x0352` | NAV1 / NAV2 active, leading 1 dropped | `nav1_frequency_hz` / `nav2_frequency_hz` | `11345` |
| `0x311E` / `0x3120` | NAV1 / NAV2 standby | `nav1_standby_frequency_hz` / `nav2_...` | `11345` |
| `0x034C` | ADF1 main 3 digits (1234.5 → `0x0234`) | `adf1_frequency_hz` (kHz) | `1234` |
| `0x0356` | ADF1 thousands digit in the high byte, fraction in the low byte (1234.5 → `0x0105`) | same | |
| `0x02D4` / `0x02D6` | ADF2, same layout as `0x034C` / `0x0356` (the guide marks them "FS2004 only") | `adf2_frequency_hz` | `1234` |

`DataRefs.txt` lists the ADF datarefs' unit as "10kHz", the same label the COM/NAV datarefs use. Whether X-Plane actually holds 362 kHz as `362` or as something else needs checking in the sim (DataRefTool) before the ADF expressions are written.

The guide has two other BCD fields, both version numbers rather than radio values. `0x3304`/`0x3306` (FSUIPC version) is already served as a hex `static_value`. `0x3322` (WideServer version) isn't relevant to this plugin. Neither needs the operators. The guide's other table (`panels-token`, 182 rows) has no BCD fields.

## Goals / Non-Goals

**Goals:**

- BCD offsets can be read and written using only expression mappings.

**Non-Goals:**

- A `bcd` mapping field. It was considered and dropped: it would add a second code path (loader, plugin, debugger) to save a few lines per mapping.
- 8.33 kHz COM channels. `0x034E` only holds 10 kHz resolution, and the 8.33-capable offsets are plain integers in Hz.

## Decisions

### Operators only

Each BCD offset is an expression mapping:

```toml
datarefs    = { C = "sim/cockpit2/radios/actuators/com1_frequency_hz" }
expr        = "$C 10000 - tobcd"
write_exprs = { C = "$value frombcd 10000 +" }
```

The read and write expressions have to be kept consistent by hand. The shipped mappings set the pattern, and the `mappings.toml` header documents it.

### Pure functions behind the operators

`uipc-expr` gets `pub fn to_bcd(f64) -> f64` and `pub fn from_bcd(f64) -> f64`, which give `NaN` on invalid input. The `Op::ToBcd`/`Op::FromBcd` evaluation arms call them. They are public so they can be tested on their own, and reused if something outside expressions ever needs them.

### Limits

Both operators round their input first. `tobcd` allows up to 13 decimal digits, the most that stays exact in an `f64` (4 × 13 = 52 bits). `frombcd` accepts values up to 2^53. Invalid input gives `NaN` rather than clamping or skipping bad nibbles, so a malformed client write (such as `0x12AB`) is blocked by the existing all-or-nothing write rule instead of setting a wrong frequency.

### Leading 1 on COM/NAV

`12345 tobcd` is `0x12345`, and a `u16` would wrap it to `0x2345` anyway. The shipped mappings still subtract 10000 explicitly, so the read and write expressions are visibly inverses of each other.

## Risks / Trade-offs

- [Read and write expressions drift apart] → One shipped pattern per offset kind, documented in the header. Each spec scenario is a round trip.
- [The X-Plane radio datarefs are 10 kHz units, so an 8.33 kHz frequency such as 118.005 reads as 11800] → FSUIPC's offset format has the same limit. It's noted in the mapping comments.
- [Writing `com*_frequency_hz` may be overridden by a third-party aircraft's own radio logic] → Same as any dataref write. Users can add `write_commands` per aircraft.

## Migration Plan

Additive. The `0x0354` mapping changes from serving the raw code (`2770` = `0x0AD2`) to serving BCD. That fixes a bug: clients already expect BCD there.
