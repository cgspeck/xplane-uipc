# Spec Delta

## Purpose

Converts offset values to and from binary coded decimal (BCD), which FSUIPC uses for radio frequencies and the transponder code, so expression mappings can read and write these offsets as X-Plane decimal values.

## ADDED Requirements

### Requirement: tobcd operator

The `uipc-expr` evaluator SHALL support a unary `tobcd` operator. It SHALL round its operand to the nearest integer (half away from zero) and give the number whose hexadecimal digits are that integer's decimal digits. The result SHALL be `NaN` when the operand is negative or not finite, or when the rounded value has more than 13 decimal digits.

#### Scenario: Transponder code

- **WHEN** the expression `2770 tobcd` is evaluated
- **THEN** the result is `10096` (`0x2770`)

#### Scenario: Fraction rounds first

- **WHEN** the expression `2344.5 tobcd` is evaluated
- **THEN** the result is `9029` (`0x2345`)

#### Scenario: Zero

- **WHEN** the expression `0 tobcd` is evaluated
- **THEN** the result is `0`

#### Scenario: Invalid operand

- **WHEN** `-1 tobcd` or `10000000000000 tobcd` (14 digits) is evaluated
- **THEN** the result is `NaN`

### Requirement: frombcd operator

The `uipc-expr` evaluator SHALL support a unary `frombcd` operator. It SHALL round its operand to the nearest integer and read that integer's hexadecimal digits as decimal digits. The result SHALL be `NaN` when the operand is negative or not finite, when the rounded value is above 2^53, or when any hexadecimal digit is above 9.

#### Scenario: COM frequency

- **WHEN** the expression `9029 frombcd` is evaluated
- **THEN** the result is `2345`

#### Scenario: Round trip

- **WHEN** the expression `7700 tobcd frombcd` is evaluated
- **THEN** the result is `7700`

#### Scenario: Not valid BCD

- **WHEN** the expression `4779 frombcd` (`0x12AB`) is evaluated
- **THEN** the result is `NaN`

### Requirement: BCD offsets round-trip through expression mappings

An expression mapping SHALL be able to serve a decimal dataref as BCD with `tobcd` in `expr`, and to write it back with `frombcd` in `write_exprs`. A client write that isn't valid BCD SHALL write nothing, by the existing rule that a non-finite write result blocks the whole write.

#### Scenario: Read COM frequency

- **GIVEN** a `u16` mapping with `datarefs = { C = "sim/cockpit2/radios/actuators/com1_frequency_hz" }` and `expr = "$C 10000 - tobcd"`
- **AND** `C` is `12345` (123.45 MHz)
- **THEN** the offset is served as `0x2345`

#### Scenario: Write COM frequency

- **GIVEN** that mapping with `writable = true` and `write_exprs = { C = "$value frombcd 10000 +" }`
- **WHEN** a client writes `u16` `0x2250`
- **THEN** `com1_frequency_hz` is set to `12250` (122.50 MHz)

#### Scenario: Invalid BCD write

- **GIVEN** that writable mapping
- **WHEN** a client writes `0x12AB`
- **THEN** `com1_frequency_hz` is not written and a warning naming the offset is logged
