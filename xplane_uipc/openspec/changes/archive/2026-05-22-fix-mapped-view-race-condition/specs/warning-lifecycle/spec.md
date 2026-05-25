## ADDED Requirements

### Requirement: WarnedSet supports clearing individual offset+category bits
The `WarnedSet` SHALL provide a `clear_key(key, category)` method that clears the warned bit for a specific offset and category combination, allowing that offset to be re-evaluated on the next access.

#### Scenario: Cleared key can warn again
- **WHEN** `check_and_set(0x3304, ReadNotExist)` has been called (bit set) and then `clear_key(0x3304, ReadNotExist)` is called
- **THEN** the next `check_and_set(0x3304, ReadNotExist)` returns `true` (bit was cleared)

#### Scenario: Clearing one key does not affect others
- **WHEN** `check_and_set(0x3304, ReadNotExist)` and `check_and_set(0x3308, ReadNotExist)` have both been called, then `clear_key(0x3304, ReadNotExist)` is called
- **THEN** `check_and_set(0x3304, ReadNotExist)` returns `true` but `check_and_set(0x3308, ReadNotExist)` returns `false`

#### Scenario: Clearing one category does not affect others
- **WHEN** `check_and_set(0x3304, ReadNotExist)` and `check_and_set(0x3304, WriteNotExist)` have both been called, then `clear_key(0x3304, ReadNotExist)` is called
- **THEN** `check_and_set(0x3304, ReadNotExist)` returns `true` but `check_and_set(0x3304, WriteNotExist)` returns `false`

### Requirement: Table population clears corresponding ReadNotExist warnings
After the table is populated (either via `populate_table()` or `update()`), the system SHALL clear `ReadNotExist` warnings for all offsets that now have entries in the table.

#### Scenario: Initial population clears pre-existing warnings
- **WHEN** an IPC client reads offset `0x3304` before table population (warning logged) and then `populate_table()` adds an entry for `0x3304`
- **THEN** the `ReadNotExist` bit for `0x3304` is cleared so subsequent reads do not carry the stale warning state

#### Scenario: Flight loop update clears warnings for newly-resolved datarefs
- **WHEN** a dataref-based mapping was not in the table initially (dataref not yet available) and `update()` successfully adds it
- **THEN** the `ReadNotExist` bit for that offset is cleared
