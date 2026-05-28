## Tasks

### 1. Add new operators to the parser and evaluator
**File:** `uipc-expr/src/lib.rs`

- Add `Dup`, `Swap`, `Floor`, `Ceil`, `Min`, `Max`, `Neg`, `Sqrt`, `Not`, `Sin`, `Cos`, `Atan2` to the `Op` enum
- Add parse arms: `"dup"`, `"swap"`, `"floor"`, `"ceil"`, `"min"`, `"max"`, `"neg"`, `"sqrt"`, `"not"`, `"sin"`, `"cos"`, `"atan2"`
- Add `"E"` as a `Token::Num(std::f64::consts::E)` literal (same pattern as `PI`)
- Add eval arms:
  - `Dup`: peek top, push copy (no-op if stack empty)
  - `Swap`: pop two, push in reversed order (no-op if <2)
  - `Floor`/`Ceil`/`Neg`/`Sqrt`/`Not`/`Sin`/`Cos`: unary pop-push
  - `Min`/`Max`/`Atan2`: binary pop-pop-push
  - `Sqrt` of negative → 0.0 (same safety pattern as div-by-zero)
- Add Display arms for all new ops
- Add `"E"` handling in `token_strs()`

### 2. Add tests for all new operators
**File:** `uipc-expr/src/lib.rs`

Tests to add:

**Stack manipulation:**
- `test_dup` — `5 dup *` = 25.0
- `test_dup_stack_underflow` — `dup` on empty stack = 0.0
- `test_swap` — `3 5 swap -` = 2.0
- `test_swap_stack_underflow` — `5 swap` with only one element

**Math:**
- `test_floor` — `3.7 floor` = 3.0
- `test_floor_negative` — `-3.2 floor` = -4.0
- `test_ceil` — `3.2 ceil` = 4.0
- `test_ceil_negative` — `-3.7 ceil` = -3.0
- `test_min` — `3 5 min` = 3.0
- `test_max` — `3 5 max` = 5.0
- `test_min_equal` — `4 4 min` = 4.0
- `test_neg` — `5 neg` = -5.0
- `test_neg_negative` — `-3 neg` = 3.0
- `test_sqrt` — `9 sqrt` = 3.0
- `test_sqrt_negative` — `-4 sqrt` = 0.0

**Logic:**
- `test_not_zero` — `0 not` = 1.0
- `test_not_nonzero` — `5 not` = 0.0
- `test_not_negative` — `-1 not` = 0.0

**Trig:**
- `test_sin_zero` — `0 sin` = 0.0
- `test_sin_pi_half` — `PI 2 / sin` ≈ 1.0
- `test_cos_zero` — `0 cos` = 1.0
- `test_cos_pi` — `PI cos` ≈ -1.0
- `test_atan2_basic` — `1 1 atan2` ≈ PI/4
- `test_atan2_zero` — `0 1 atan2` = 0.0

**Constants:**
- `test_e` — `E` ≈ 2.71828

**Display roundtrip:**
- `test_display_new_ops` — verify all new ops survive parse→display→parse roundtrip

### 3. Add UTC offset midnight-crossover integration test
**File:** `uipc-expr/src/lib.rs`

Add a test that evaluates the UTC offset expression using `dup`/`swap`:
```
$zh 60 * $zm + $lh 60 * $lm + - dup dup 720 > 1440 0 ? swap -720 < 1440 0 ? swap - +
```

Test cases:
- Normal positive offset (UTC+5:30): zh=10, zm=0, lh=15, lm=30 → -330
- Normal negative offset (UTC-5): zh=15, zm=0, lh=10, lm=0 → 300
- Midnight crossover (UTC+10): zh=22, zm=0, lh=8, lm=0 → -600
- Midnight crossover negative (UTC-10): zh=2, zm=0, lh=16, lm=0 → 600

### 4. Update README documentation
**File:** `uipc-expr/README.md`

- Add `dup`, `swap` under a new "Stack Manipulation" section
- Add `floor`, `ceil`, `min`, `max`, `neg`, `sqrt` to the Arithmetic/Unary section
- Add `not` to a Logic section (or alongside bitwise)
- Add `sin`, `cos`, `atan2` under a new "Trigonometry" section
- Add `E` to the Literals table

### 5. Update expr-calculator commands and help text
**File:** `expr-calculator/src/main.rs`

- Add all new operator strings to the `commands` vec
- Add `"E"` to the commands vec
- Update the help text to mention the new operator categories

### 6. Add UTC offset mapping to mappings.toml
**File:** `xplane_uipc/mappings.toml`

Replace the commented-out `0x0246` block with an active mapping:

```toml
[[mapping]]
offset      = 0x0246
fsuipc_type = "i16"
datarefs    = { zh = "sim/cockpit2/clock_timer/zulu_time_hours", zm = "sim/cockpit2/clock_timer/zulu_time_minutes", lh = "sim/cockpit2/clock_timer/local_time_hours", lm = "sim/cockpit2/clock_timer/local_time_minutes" }
expr        = "$zh 60 * $zm + $lh 60 * $lm + - dup dup 720 > 1440 0 ? swap -720 < 1440 0 ? swap - +"
```

Local time offset from Zulu in minutes, +ve = behind. Handles midnight crossover.
