## ADDED Requirements

### Requirement: Writable mappings are reversible

The loader SHALL reject a mapping with `writable = true`, with a load error naming the offset, when:

- it is a single-dataref mapping with `scale = 0`, because its read is the constant `offset_add`
- it is an expression mapping without `write_exprs`

A read-only single-dataref mapping with `scale = 0` SHALL still load.

#### Scenario: Writable zero scale

- **WHEN** a mapping has `dataref = "..."`, `scale = 0` and `writable = true`
- **THEN** the mapping is not loaded and a load error says a writable mapping can't have `scale = 0`

#### Scenario: Read-only zero scale

- **WHEN** a mapping has `dataref = "..."`, `scale = 0`, `offset_add = 5` and no `writable`
- **THEN** the mapping loads and reads as `5`

### Requirement: Single-dataref writes reverse the read

A write to a writable single-dataref mapping SHALL set the dataref to `(value - offset_add) / scale`, where `value` is the client's written value decoded by the offset's type.

#### Scenario: Scaled write

- **GIVEN** a writable `u16` mapping with `scale = 100` and `offset_add = 0`
- **WHEN** a client writes `250`
- **THEN** the dataref is set to `2.5`

### Requirement: Writes that reach no dataref are reported

A single-dataref write whose dataref is unavailable at runtime SHALL be skipped. The first such skip for a mapping since mappings were last loaded SHALL log a warning naming the offset and the dataref, and later skips SHALL be logged at debug level. The plugin SHALL log that a write succeeded only when at least one dataref was written.

#### Scenario: Missing add-on dataref

- **GIVEN** a writable single-dataref mapping whose dataref belongs to an add-on that isn't loaded
- **WHEN** a client writes the offset twice
- **THEN** no dataref is written, one warning names the offset and the dataref, and the second skip is logged at debug level
- **AND** no "Wrote value" message is logged for either write
