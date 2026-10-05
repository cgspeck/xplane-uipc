# Offset Value Semantics Specification

## Purpose

How mapped values are converted to and from FSUIPC types, when an offset is served, and how client reads and writes are bounded and decoded.

## Requirements

### Requirement: Offsets without a current value are not served

Each update cycle, the value table SHALL contain an entry for an offset only if its mapping produced a value in that cycle, or deliberately retained its previous value. All other entries SHALL be removed.

#### Scenario: Mapping removed on reload

- **WHEN** a mapping for offset `0x0D0C` is served, then removed from mappings.toml and mappings are reloaded
- **THEN** after the next update cycle, a read of `0x0D0C` takes the "not in table" path

#### Scenario: Dataref becomes unavailable

- **WHEN** a simple mapping's dataref handle is null or its read fails
- **THEN** the offset is removed from the table and from the active/writable sets on that cycle

#### Scenario: update_if_expr false retains the last value

- **WHEN** a mapping with `update_if_expr` has been served with value `V`, and on a later cycle `update_if_expr` evaluates to `<= 0`
- **THEN** the offset continues to be served with value `V` and remains active (and writable if configured)

#### Scenario: update_if_expr false before any value serves zero

- **WHEN** a mapping with `update_if_expr` has never produced a value since the plugin started (e.g. `0x030C` touchdown vertical speed after starting on the ground) and `update_if_expr` evaluates to `<= 0`
- **THEN** the offset is served as zero of its declared type and is active, as FSUIPC does at startup

### Requirement: Float to integer conversion rounds and wraps

Converting a mapping's f64 result to an integer FSUIPC type SHALL round to the nearest integer (half away from zero). Results outside the range of an integer type up to 32 bits SHALL wrap using two's complement. Non-finite results SHALL produce no value.

#### Scenario: Near-integer value rounds up

- **WHEN** an `i32` mapping evaluates to `1023.9999`
- **THEN** the served value is `1024`

#### Scenario: Negative value in unsigned offset wraps

- **WHEN** a `u16` mapping evaluates to `-1.0`
- **THEN** the served value is `0xFFFF`

#### Scenario: NaN produces no value

- **WHEN** a mapping evaluates to `NaN` or an infinity
- **THEN** the offset is treated as having no value for that cycle

### Requirement: Writes are decoded by the offset's type

A client write SHALL be decoded according to the type of the offset's current table entry. The write's `nBytes` SHALL equal that type's width; otherwise the write SHALL be rejected and not forwarded.

#### Scenario: Signed 16-bit write

- **WHEN** a client writes `0xFFFF` (2 bytes) to a writable `i16` offset
- **THEN** the value forwarded to X-Plane is `-1`

#### Scenario: Float32 write

- **WHEN** a client writes the 4-byte encoding of `1.5f32` to a writable `f32` offset
- **THEN** the value forwarded is `1.5`

#### Scenario: Size mismatch rejected

- **WHEN** a client writes 2 bytes to a writable `u32` offset
- **THEN** no write is forwarded and an error is counted

### Requirement: Integer dataref writes round

Writing to an integer or integer-array dataref SHALL round the X-Plane-side value to the nearest integer.

#### Scenario: Inverse scale produces a fraction

- **WHEN** a write's inverse transform yields `2.9999` for an `int` dataref
- **THEN** the dataref is set to `3`

### Requirement: Reads never exceed the requested size

Answering a read request SHALL write at most `nBytes` bytes into the record's payload.

#### Scenario: Narrow read of a wide offset

- **WHEN** a client reads 2 bytes from an offset holding `u32` `0x12345678`
- **THEN** exactly 2 bytes, `78 56`, are written and the following bytes are untouched

### Requirement: String values are bounded and terminated

A string mapping SHALL serve at most `size` bytes, always ending in a NUL. A dataref returning 0 bytes SHALL be served as an empty string.

#### Scenario: Dataref fills the whole size

- **WHEN** a `size = 8` string dataref returns 8 non-NUL bytes
- **THEN** the served value is the first 7 bytes followed by NUL

#### Scenario: Empty dataref

- **WHEN** a string dataref with a valid handle returns 0 bytes
- **THEN** the offset is served as an empty (all-NUL) string
