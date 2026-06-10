## Why

The `%` (modulo) operator in `uipc-expr` casts both operands to `i64` before computing the remainder, which truncates the fractional part. This makes expressions like `$elev 1 %` (extract fractional metres) always return `0` instead of the fractional component. The altitude mapping at offset `0x0570` depends on this and is currently broken.

## What Changes

- Change the `%` operator in `uipc-expr` from integer modulo (`(a as i64) % (b as i64)`) to floating-point remainder (`a % b` / `f64::rem`).
- Add an explicit integer-modulo operator (`imod`) for cases where integer truncation before modulo is intentional.

## Capabilities

### New Capabilities
- `float-mod-operator`: Change the `%` operator to use floating-point remainder and add an `imod` operator for explicit integer modulo.

### Modified Capabilities

## Impact

- **`uipc-expr`**: `Op::Mod` evaluation changes from integer to float semantics. Existing expressions using `%` with integer-valued operands produce the same results (e.g., `10 3 %` = `1.0` either way). Expressions relying on truncation before modulo (none known) would need to switch to `imod`.
- **`xplane_uipc/mappings.toml`**: The `0x0570` altitude fractional-metres mapping will start working correctly without any mapping file changes.
