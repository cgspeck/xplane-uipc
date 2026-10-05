## ADDED Requirements

### Requirement: Write expressions on expression mappings

An expression mapping MAY have a `write_exprs` table. Each key SHALL name an entry in the mapping's `datarefs`, and each value SHALL be an RPN expression for the value to write to that dataref when a client writes the offset. A write expression SHALL be able to use `$value`, the client's written value decoded by the offset's type, and every name in the mapping's `datarefs`, holding that dataref's current value. Unavailable datarefs SHALL read as `0.0`.

#### Scenario: Write fans out to several datarefs

- **GIVEN** a writable `u16` mapping with `datarefs = { Nav = "...navigation_lights_on", Bcn = "...beacon_on" }` and `write_exprs = { Nav = "$value 1 & 0 !=", Bcn = "$value 2 & 0 !=" }`
- **WHEN** a client writes `2`
- **THEN** `navigation_lights_on` is set to `0` and `beacon_on` is set to `1`

#### Scenario: Write uses a current dataref value

- **GIVEN** a writable mapping with `datarefs = { Z = "sim/time/zulu_time_sec" }` and `write_exprs = { Z = "$value 3600 * $Z 3600 % +" }`
- **AND** `zulu_time_sec` is `45296.5` (12:34:56.5)
- **WHEN** a client writes `1`
- **THEN** `zulu_time_sec` is set to `5696.5` (01:34:56.5)

#### Scenario: Integer and indexed targets

- **WHEN** a write expression's target is an `int` dataref or uses `dataref[N]` notation
- **THEN** the result is written the same way as a single-dataref write: rounded to the nearest integer for integer datarefs, and to element `N` for indexed ones

### Requirement: Write expressions are all-or-nothing

The plugin SHALL evaluate every write expression of a mapping against one snapshot of the current dataref values before writing any of them. If any result is not a finite number, it SHALL write none of them and SHALL log a warning naming the offset. Targets SHALL be written in sorted key order. A target whose dataref is unavailable at runtime SHALL be skipped with a warning, and the other targets SHALL still be written.

#### Scenario: Non-finite result blocks the whole write

- **GIVEN** a mapping with `write_exprs = { A = "$value", B = "10 $value ^" }`
- **WHEN** a client writes `400` (so `B` overflows to infinity)
- **THEN** neither `A` nor `B` is written and a warning is logged

#### Scenario: Results don't see each other's writes

- **GIVEN** `write_exprs = { A = "$B", B = "$A" }` with `A = 1` and `B = 2`
- **WHEN** a client writes the offset
- **THEN** `A` is set to `2` and `B` is set to `1`

### Requirement: Write expressions are validated at load

The loader SHALL reject a mapping, with a load error naming the offset, when:

- `write_exprs` is present on a mapping without `expr`
- `write_exprs` is present without `writable = true`
- `write_exprs` is empty
- a `write_exprs` key isn't a name in `datarefs`
- a write expression fails to parse
- a write expression uses a variable that is neither `value` nor a name in `datarefs`
- `write_exprs` is present and `datarefs` has an entry named `value`

The loader SHALL also reject a writable expression mapping that has no `write_exprs`. A read-only mapping without `write_exprs` SHALL load exactly as before.

#### Scenario: Unknown target

- **WHEN** a mapping has `datarefs = { SB = "..." }` and `write_exprs = { Sb = "$value" }`
- **THEN** the mapping is not loaded and a load error mentions `Sb`

#### Scenario: Unknown variable

- **WHEN** a write expression is `"$valu 2 *"`
- **THEN** the mapping is not loaded and a load error mentions `valu`

#### Scenario: Not writable

- **WHEN** a mapping has `write_exprs` but no `writable = true`
- **THEN** the mapping is not loaded and a load error says `write_exprs` requires `writable = true`

#### Scenario: Writable without write expressions

- **WHEN** an expression mapping has `writable = true` and no `write_exprs`
- **THEN** the mapping is not loaded and a load error says a writable expression mapping needs `write_exprs`

#### Scenario: Expression mapping without datarefs

- **WHEN** a mapping has `expr = "5"`, no `datarefs`, `writable = true` and `write_exprs = { X = "$value" }`
- **THEN** the mapping is not loaded and a load error mentions `X`

### Requirement: Writability warnings cover write targets

When mappings are resolved, the plugin SHALL warn once for each write target whose dataref X-Plane reports as read-only.

#### Scenario: Read-only target

- **WHEN** a mapping's `write_exprs` targets `sim/cockpit2/clock_timer/zulu_time_hours`
- **THEN** a warning names the offset and that dataref as read-only

### Requirement: Unavailable write targets are reported once per load

When a write skips a target because its dataref is unavailable, the plugin SHALL log a warning naming the offset and the dataref the first time this happens for that mapping since mappings were last loaded. Later skips SHALL be logged at debug level.

#### Scenario: Repeated writes to a missing add-on dataref

- **GIVEN** a write target whose dataref belongs to an add-on that isn't loaded
- **WHEN** a client writes the offset three times
- **THEN** one warning is logged, and the other two skips are logged at debug level
- **AND** after "Reload Mappings", the next skip logs a warning again
