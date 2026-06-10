## Context

The `uipc-expr` crate implements a stack-based RPN expression evaluator. The `%` operator currently casts both operands to `i64` before computing the remainder (`(a as i64) % (b as i64)`). This is consistent with the `\` (integer division) operator but inconsistent with how FSUIPC's RPN evaluator handles `%`, which operates on floats. The altitude mapping at `0x0570` uses `$elev 1 %` to extract fractional metres, which always evaluates to `0` because `304.75 as i64` truncates to `304` before `304 % 1 = 0`.

## Goals / Non-Goals

**Goals:**
- Fix `%` to use floating-point remainder so `304.75 1 %` returns `0.75`
- Preserve integer modulo as an explicit operator (`imod`) for expressions that need it
- Maintain backward compatibility for integer-valued operands (e.g., `10 3 %` still returns `1.0`)

**Non-Goals:**
- Changing any other operator semantics
- Modifying the `\` (integer division) operator
- Changing the mapping file format

## Decisions

### 1. Use Rust's `f64 %` (remainder) for the `%` operator

**Choice**: Replace `((a as i64) % (b as i64)) as f64` with `a % b` (Rust's `f64::rem`).

**Rationale**: Rust's `%` on `f64` computes the IEEE 754 remainder. For integer-valued floats the result is identical to integer modulo (e.g., `10.0 % 3.0 = 1.0`), so no existing expressions break. Expressions like `$elev 1 %` that need the fractional part only work with float remainder.

**Alternative considered**: Adding a new `fmod` operator and keeping `%` as integer. Rejected because float-mod is the more natural expectation for `%` in a float-based evaluator, and no known expressions rely on the truncation behaviour.

### 2. Add `imod` as the explicit integer-modulo operator

**Choice**: Add `Op::IMod` with the current `(a as i64) % (b as i64)` semantics, parsed from the token `imod`.

**Rationale**: Preserves the ability to do integer modulo explicitly if needed, following the same pattern as `\` (integer division) vs `/` (float division).

## Risks / Trade-offs

- **[Semantic change to `%`]** → Low risk. For integer-valued operands the results are identical. No expressions in `mappings.toml` use `%` with non-integer divisors where integer truncation is desired. The `imod` escape hatch exists if needed.
- **[Division by zero]** → The existing guard (`b.abs() < 1e-300 → 0.0`) will be preserved for both `%` and `imod`.
