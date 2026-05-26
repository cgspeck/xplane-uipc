## MODIFIED Requirements

### Requirement: Parse offset definitions from text file

**Change:** Inline `#` comments are now stripped before parsing fields.

**Before:** `0x02BC,i32      # comment` failed with unknown type error because `#` was parsed as part of the type field.

**After:** Characters from the first `#` to end of line are stripped before field splitting. `0x02BC,i32      # comment` parses as address `0x02BC`, type `i32`.

#### Scenario: Parse line with inline comment
- **WHEN** the input file contains `0x02BC,i32      # Indicated airspeed`
- **THEN** the system SHALL parse the offset as address 0x02BC, type i32, size 4

#### Scenario: Inline comment does not affect subsequent lines
- **WHEN** the input file contains `0x02BC,i32      # comment\n0x0D0C,u16`
- **THEN** both lines SHALL parse correctly
