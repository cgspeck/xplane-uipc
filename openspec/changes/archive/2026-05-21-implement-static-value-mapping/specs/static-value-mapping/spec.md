## ADDED Requirements

### Requirement: Mapping can specify a static constant value

A mapping in `mappings.toml` SHALL support a `static_value` field that specifies a fixed `f64` constant, requiring no dataref or expression evaluation.

#### Scenario: TOML entry with static_value produces MappingSource::Static

- **WHEN** a TOML mapping entry includes `static_value = 42.0` without `dataref` or `expr`
- **THEN** `load_mappings()` SHALL produce a `DatarefMapping` with `MappingSource::Static { static_value: 42.0 }`

#### Scenario: static_value with only offset and fsuipc_type

- **WHEN** a TOML entry has only `offset`, `fsuipc_type`, and `static_value`
- **THEN** `load_mappings()` SHALL succeed and produce a valid `DatarefMapping`

#### Scenario: Negative and zero static values

- **WHEN** `static_value` is `0.0` or a negative value like `-1.5`
- **THEN** `load_mappings()` SHALL accept these as valid values and produce the correct `MappingSource::Static`

### Requirement: Priority among source fields

When multiple source fields are present, `load_mappings()` SHALL follow a deterministic priority: `expr` > `dataref` > `static_value`.

#### Scenario: expr and static_value both present

- **WHEN** a TOML entry has both `expr = "1 2 +"` and `static_value = 99.0`
- **THEN** `load_mappings()` SHALL create `MappingSource::Expr`, ignoring `static_value`

#### Scenario: dataref and static_value both present

- **WHEN** a TOML entry has both `dataref = "..."` and `static_value = 99.0`
- **THEN** `load_mappings()` SHALL create `MappingSource::Simple`, ignoring `static_value`

### Requirement: Error for mapping with no source

A mapping entry with none of `expr`, `dataref`, or `static_value` SHALL produce a descriptive error.

#### Scenario: No source fields produces error

- **WHEN** a TOML entry has only `offset`, `fsuipc_type`, and no `dataref`, `expr`, or `static_value`
- **THEN** `load_mappings()` SHALL return an error mentioning all three valid fields

### Requirement: Static value is writable-safe

When `writable = true` on a static value mapping, `write_xplane()` SHALL do nothing (static values are read-only by nature).

#### Scenario: Writing to static mapping is a no-op

- **WHEN** `m.write_xplane(123.0)` is called on a `ResolvedMapping` with `ResolvedSource::Static`
- **THEN** no X-Plane dataref write occurs and no error is produced
