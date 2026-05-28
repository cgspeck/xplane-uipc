## Why

Non-trivial expressions (e.g., UTC offset with midnight-crossover handling) require computing the same sub-expression multiple times because there are no stack-manipulation operators. This makes expressions unreadable and error-prone. Several other commonly-needed operations (clamping, rounding variants, trig for nav math, logical negation) are also missing.

## What Changes

Add 13 new operators to the `uipc-expr` evaluator, grouped into four categories:

### Stack manipulation

| Op | Stack effect | Description |
|---|---|---|
| `dup` | `a -- a a` | Duplicate top of stack |
| `swap` | `a b -- b a` | Swap top two values |

These are the highest-impact additions — they eliminate redundant recomputation and make every other operator more usable.

### Math

| Op | Stack effect | Description |
|---|---|---|
| `floor` | `a -- floor(a)` | Round toward negative infinity |
| `ceil` | `a -- ceil(a)` | Round toward positive infinity |
| `min` | `a b -- min(a,b)` | Minimum of two values |
| `max` | `a b -- max(a,b)` | Maximum of two values |
| `neg` | `a -- -a` | Negate (flip sign) |
| `sqrt` | `a -- sqrt(a)` | Square root (NaN→0.0 for negative inputs) |

### Logic

| Op | Stack effect | Description |
|---|---|---|
| `not` | `a -- !a` | Logical negation: 0.0→1.0, nonzero→0.0 |

### Trigonometry

| Op | Stack effect | Description |
|---|---|---|
| `sin` | `a -- sin(a)` | Sine (radians) |
| `cos` | `a -- cos(a)` | Cosine (radians) |
| `atan2` | `y x -- atan2(y,x)` | Two-argument arctangent (radians) |

### Constants

| Token | Value |
|---|---|
| `E` | Euler's number (2.71828...) |

## Scope

### In scope

- New operators in `uipc-expr/src/lib.rs` (parser, evaluator, Display)
- Comprehensive tests for each new operator (including edge cases: stack underflow, division by zero for sqrt of negative, etc.)
- An integration-style test using the UTC offset midnight-crossover expression from our discussion as a real-world validation
- Updated `uipc-expr/README.md` operator tables
- Updated `expr-calculator/src/main.rs` commands list and help text

### Out of scope

- `drop` / `over` / `rot` and other less common stack ops (can add later if needed)
- Changes to mapping parsing or plugin runtime
- New constant additions beyond `E`

## Example: UTC offset with `dup`/`swap`

Before (computing the diff 3 times):
```
$zh 60 * $zm + $lh 60 * $lm + - $zh 60 * $zm + $lh 60 * $lm + - 720 > 1440 0 ? - $zh 60 * $zm + $lh 60 * $lm + - -720 < 1440 0 ? +
```

After (compute once, `dup` twice):
```
$zh 60 * $zm + $lh 60 * $lm + - dup dup 720 > 1440 0 ? swap -720 < 1440 0 ? swap - +
```

Where `$zh`/`$zm`/`$lh`/`$lm` are shorthand for the `sim/cockpit2/clock_timer/` datarefs.

## Capabilities

### New Capabilities
*(none — this extends an existing internal crate, not a user-facing capability)*

### Modified Capabilities
*(none)*
