## ADDED Requirements

### Requirement: Modulo operator uses floating-point remainder
The `%` operator in `uipc-expr` SHALL compute the IEEE 754 floating-point remainder (`a % b` in Rust) rather than casting operands to integers first.

#### Scenario: Fractional modulo extracts fractional part
- **WHEN** the expression `304.75 1 %` is evaluated
- **THEN** the result SHALL be `0.75`

#### Scenario: Integer-valued operands produce same result as before
- **WHEN** the expression `10 3 %` is evaluated
- **THEN** the result SHALL be `1.0`

#### Scenario: Negative fractional modulo
- **WHEN** the expression `-5.3 1 %` is evaluated
- **THEN** the result SHALL be `-0.3` (IEEE 754 remainder preserves sign of dividend)

#### Scenario: Division by zero returns zero
- **WHEN** the expression `5.5 0 %` is evaluated
- **THEN** the result SHALL be `0.0`

### Requirement: Integer modulo operator imod
The `uipc-expr` evaluator SHALL support an `imod` operator that computes integer modulo by truncating both operands to `i64` before computing the remainder, matching the previous behaviour of `%`.

#### Scenario: imod truncates before modulo
- **WHEN** the expression `304.75 3 imod` is evaluated
- **THEN** the result SHALL be `1.0` (i.e., `304 % 3`)

#### Scenario: imod with integer operands
- **WHEN** the expression `10 3 imod` is evaluated
- **THEN** the result SHALL be `1.0`

#### Scenario: imod division by zero returns zero
- **WHEN** the expression `5 0 imod` is evaluated
- **THEN** the result SHALL be `0.0`

### Requirement: Token string round-trip for imod
The `imod` operator SHALL be represented as the string `"imod"` in the token display format, and SHALL be parseable from the string `"imod"`.

#### Scenario: Display and parse round-trip
- **WHEN** an expression containing `imod` is displayed and re-parsed
- **THEN** the resulting expression SHALL be semantically identical
