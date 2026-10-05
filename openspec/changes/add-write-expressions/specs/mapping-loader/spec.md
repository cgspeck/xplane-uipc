## ADDED Requirements

### Requirement: A mapping has one dataref source

The loader SHALL reject a mapping that has both `dataref` and `expr`, with a load error naming the offset. Before this change, `expr` was used and `dataref`, `scale` and `offset_add` were ignored without a warning.

#### Scenario: Both dataref and expr

- **WHEN** a mapping has `dataref = "sim/a"`, `scale = 100` and `expr = "$A"`
- **THEN** the mapping is not loaded and a load error says a mapping can't have both `dataref` and `expr`
