### Requirement: Activity counter at 0x337E

The plugin SHALL serve offset `0x337E` as an unsigned 16-bit value that increments by one on every update cycle and wraps from `0xFFFF` to `0`. It SHALL be served without any `mappings.toml` entry.

#### Scenario: Counter increments each cycle

- **WHEN** the counter holds value `N` after an update cycle
- **THEN** after the next update cycle a 2-byte read of `0x337E` returns `N + 1`

#### Scenario: Counter wraps at max value

- **WHEN** the counter holds `0xFFFF` after an update cycle
- **THEN** after the next update cycle a read of `0x337E` returns `0`

#### Scenario: Served with an empty mapping file

- **WHEN** `mappings.toml` contains no mappings and an update cycle has run
- **THEN** `0x337E` is active in the value table with type `u16`

### Requirement: Built-in offsets are read-only

Built-in offsets SHALL NOT be writable by clients.

#### Scenario: Client write rejected

- **WHEN** a client writes 2 bytes to `0x337E`
- **THEN** the write takes the non-writable path, nothing is forwarded to the flight loop, and the counter keeps incrementing from its own value

### Requirement: Built-in offsets take precedence over mappings

A `mappings.toml` entry whose offset is a built-in offset SHALL be ignored, and a warning naming the offset SHALL be logged when mappings are resolved.

#### Scenario: Colliding mapping ignored

- **WHEN** `mappings.toml` contains a mapping for offset `0x337E`
- **THEN** the mapping is not resolved, a warning is logged, and `0x337E` serves the activity counter

### Requirement: Counter persists across mapping reloads

The activity counter SHALL NOT reset when mappings are reloaded. It SHALL start at `0` when the plugin starts.

#### Scenario: Reload mappings

- **WHEN** the counter holds `N` and the user selects "Reload Mappings"
- **THEN** the next update cycle serves `N + 1`
