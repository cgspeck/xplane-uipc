# uipc-expr

A stack-based RPN (Reverse Polish Notation) expression parser and evaluator for the X-Plane UIPC ecosystem.

## Usage

```rust
use std::collections::HashMap;
use uipc_expr::Expr;

let expr = Expr::parse("$IAS 128 *").unwrap();
let mut vars = HashMap::new();
vars.insert("IAS".to_string(), 250.0);
let result = expr.eval(&vars);
assert_eq!(result, 32000.0);
```

## Expression Syntax

Tokens are separated by whitespace.

### Literals

| Token | Description |
|---|---|
| `42` | Any f64 number |
| `PI` | π constant |
| `E` | Euler's number (2.71828...) |
| `$name` | Variable reference (fetched from the vars map; missing vars default to 0.0) |

### Operators

#### Arithmetic

| Op | Example | Result |
|---|---|---|
| `+` | `3 4 +` | 7.0 |
| `-` | `10 3 -` | 7.0 |
| `*` | `6 7 *` | 42.0 |
| `/` | `10 3 /` | 3.333... (division by near-zero returns 0.0) |
| `\` | `123 10 \` | 12.0 (integer division; near-zero returns 0.0) |
| `%` | `304.75 1 %` | 0.75 (modulo on f64 cast; near-zero returns 0.0) |
| `imod` | `10.25 3 %` | 1.0 (modulo on i64 cast; near-zero returns 0.0) |
| `^` | `2 3 ^` | 8.0 (power) |

#### Comparison

All comparisons return `1.0` (true) or `0.0` (false).

| Op | Example | Result |
|---|---|---|
| `==` | `3 3 ==` | 1.0 |
| `!=` | `3 4 !=` | 1.0 |
| `<` | `3 4 <` | 1.0 |
| `>` | `4 3 >` | 1.0 |
| `<=` | `3 4 <=` | 1.0 |
| `>=` | `4 3 >=` | 1.0 |

#### Bitwise

Operands are cast to `i64`, the operation is applied, then the result is cast back to `f64`.

| Op | Example | Result |
|---|---|---|
| `&` | `7 2 &` | 2.0 |
| `\|` | `1 2 \|` | 3.0 |

#### Unary

| Op | Example | Result |
|---|---|---|
| `abs` | `-5 abs` | 5.0 |
| `round` | `3.7 round` | 4.0 |
| `floor` | `3.7 floor` | 3.0 (toward negative infinity) |
| `ceil` | `3.2 ceil` | 4.0 (toward positive infinity) |
| `neg` | `5 neg` | -5.0 (flip sign) |
| `sqrt` | `9 sqrt` | 3.0 (negative inputs return 0.0) |
| `not` | `0 not` | 1.0 (logical negation: 0.0→1.0, nonzero→0.0) |

#### Binary coded decimal

FSUIPC stores radio frequencies and the transponder code as binary coded decimal (BCD): each hex digit holds one decimal digit, so squawk 2770 is `0x2770`. Both operators round their operand first, and give `NaN` for input they can't convert. A `NaN` read means the offset isn't served, and a `NaN` write result blocks the whole write (see [Write expressions](#write-expressions)).

| Op | Example | Result |
|---|---|---|
| `tobcd` | `2770 tobcd` | 10096.0 (`0x2770`). `NaN` if negative, not finite, or more than 13 digits |
| `frombcd` | `9029 frombcd` | 2345.0 (`0x2345` → 2345). `NaN` if negative, not finite, above 2^53, or a hex digit is above 9 (`0x12AB`) |

#### Binary (min/max)

| Op | Example | Result |
|---|---|---|
| `min` | `3 5 min` | 3.0 |
| `max` | `3 5 max` | 5.0 |

#### Trigonometry

All trig functions operate in radians.

| Op | Example | Result |
|---|---|---|
| `sin` | `PI 2 / sin` | 1.0 |
| `cos` | `0 cos` | 1.0 |
| `atan2` | `1 1 atan2` | 0.7854... (π/4) |

#### Stack Manipulation

| Op | Stack effect | Description |
|---|---|---|
| `dup` | `a -- a a` | Duplicate top of stack |
| `swap` | `a b -- b a` | Swap top two values |

#### Ternary

| Op | Stack order | Description |
|---|---|---|
| `?` | `cond then else ?` | If `cond != 0.0`, pushes `then`; otherwise pushes `else` |

Example: `$enable 42 0 ?` — returns 42 when `$enable` is non-zero, 0 otherwise.

## Write expressions

In `mappings.toml`, an expression mapping with `writable = true` turns a client's write into dataref writes through `write_exprs`. That's one expression per dataref to write, keyed by a name from the mapping's `datarefs`. The evaluator is the same as for reads, with two kinds of variable:

- `$value` is the number the client wrote, in FSUIPC units (the same number a read would give).
- Every name in `datarefs` holds that dataref's current value. Unavailable datarefs read as `0.0`.

Lights (`0x0D0C`): one write sets a switch per bit.

```toml
datarefs    = { Nav = "sim/cockpit2/switches/navigation_lights_on", Bcn = "sim/cockpit2/switches/beacon_on" }
expr        = "$Nav 1 * $Bcn 2 * +"
writable    = true
write_exprs = { Nav = "$value 1 & 0 !=", Bcn = "$value 2 & 0 !=" }
```

Writing `2` sets `navigation_lights_on` to `0` and `beacon_on` to `1`.

Zulu hour (`0x023B`): the hour dataref is read-only, so the write merges the new hour into `sim/time/zulu_time_sec` and keeps the minutes and seconds.

```toml
datarefs    = { H = "sim/cockpit2/clock_timer/zulu_time_hours", Z = "sim/time/zulu_time_sec" }
expr        = "$H"
writable    = true
write_exprs = { Z = "$value 3600 * $Z 3600 % +" }
```

With `zulu_time_sec` at `45296.5` (12:34:56.5), writing `1` sets it to `5696.5` (01:34:56.5).

BCD offsets (COM1 `0x034E`): FSUIPC drops the leading 1 of the frequency, so 123.45 MHz is `0x2345`. The read takes off the 10000 before encoding, and the write puts it back after decoding.

```toml
datarefs    = { C = "sim/cockpit2/radios/actuators/com1_frequency_hz" }   # 12345 = 123.45 MHz
expr        = "$C 10000 - tobcd"
writable    = true
write_exprs = { C = "$value frombcd 10000 +" }
```

Reading gives `0x2345`. Writing `0x2250` sets the radio to 122.50 MHz. A write that isn't valid BCD, such as `0x12AB`, gives `NaN`, so nothing is written.

Rules:

- **All or nothing.** Every write expression is evaluated against the same snapshot of the datarefs before anything is written. If any result isn't a finite number, nothing is written and a warning is logged. So `{ A = "$B", B = "$A" }` swaps the two values.
- **Sorted order.** Targets are written in sorted key order. Integer datarefs are rounded, and `dataref[N]` targets write element `N`.
- **Checked at load.** Unlike read expressions, where an unknown variable is quietly `0.0`, a write expression that uses a name other than `value` or a `datarefs` name is a load error. So is a target that isn't a `datarefs` name, an empty `write_exprs`, a `datarefs` entry called `value`, and `writable = true` with neither `write_exprs` nor `write_commands`.
- **Unavailable targets.** A dataref or command that is unavailable is skipped, and the others are still written or run. The first skip of each one after mappings load logs an error, and later ones log at debug.

### Write commands

Some aircraft, such as the Zibo 738, override their switch datarefs every frame, so a written value reverts straight away. They respond only to X-Plane commands, and often only to toggles. `write_commands` covers them: each key is a command path, and each value is an expression with the same variables as `write_exprs`. The result is how many times to run the command: it's rounded and clamped to 0–10, and 0 means don't run it.

Each command's expression picks out its own part of the written value and compares it with the switch's current state. In the lights offset, the landing lights are bit 4 (value 4):

```toml
datarefs       = { Land = "sim/cockpit2/switches/landing_lights_on", ZLand = "<add-on landing light state>" }
expr           = "$Land 4 *"
writable       = true
write_exprs    = { Land = "$value 4 & 0 !=" }                                    # default aircraft
write_commands = { "<add-on landing light toggle>" = "$value 4 & 0 != $ZLand 0 != !=" }
```

Writing `4` sets `landing_lights_on` to 1. It also runs the toggle once if `ZLand` is 0, and not at all if the add-on's lights are already on. With separate on and off commands, give each its own entry:

```toml
write_commands = { "<on>"  = "$value 4 & 0 != $ZLand 0 == &",     # want on, currently off
                   "<off>" = "$value 4 & 0 == $ZLand 0 != &" }    # want off, currently on
```

Command expressions are evaluated in the same snapshot as `write_exprs`. A non-finite result in either table stops the whole write. Datarefs are written first, then commands run in sorted path order. Commands are looked up when mappings load (and again when an aircraft loads). A command an add-on registers after that isn't found until the next load.

## API

- `Expr::parse(src)` — Parse a string into an `Expr`.
- `expr.eval(&vars)` — Evaluate the expression with the given variable map.
- `expr.vars()` — Return all variable names referenced in the expression.
- `expr.token_strs()` — Return the tokens as a vector of strings.
- `expr.to_string()` — Reconstruct the expression string from tokens.

## Safety

- Division by near-zero (< 1e-300) silently returns `0.0`.
- Square root of negative silently returns `0.0`.
- Missing variables silently default to `0.0`.
- Stack underflow for operators is silently ignored.
- Empty expressions evaluate to `0.0`.
- `tobcd` and `frombcd` return `NaN` for input they can't convert, rather than a substitute value.

## License

GNU LGPLv3.
