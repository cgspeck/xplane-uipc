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

Rules:

- **All or nothing.** Every write expression is evaluated against the same snapshot of the datarefs before anything is written. If any result isn't a finite number, nothing is written and a warning is logged. So `{ A = "$B", B = "$A" }` swaps the two values.
- **Sorted order.** Targets are written in sorted key order. Integer datarefs are rounded, and `dataref[N]` targets write element `N`.
- **Checked at load.** Unlike read expressions, where an unknown variable is quietly `0.0`, a write expression that uses a name other than `value` or a `datarefs` name is a load error. So is a target that isn't a `datarefs` name, an empty `write_exprs`, a `datarefs` entry called `value`, and `writable = true` without `write_exprs`.
- **Unavailable datarefs.** A target whose dataref is unavailable is skipped, and the others are still written. The first skip after loading mappings logs a warning, and later ones log at debug.

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

## License

GNU LGPLv3.
