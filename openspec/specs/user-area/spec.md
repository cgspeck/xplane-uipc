# user-area Specification

## Purpose

FSUIPC's 64 bytes at `0x66C0`–`0x66FF`, "free for general use". Clients store their own data there and read it back. `ipc_host` holds the area as plain memory, shared by all clients, so whatever was written is what's read, as in FSUIPC.

## Requirements

### Requirement: User area stores client writes

The server SHALL hold a 64-byte user area for offsets `0x66C0`–`0x66FF`, shared by all clients. A write record whose `dwOffset` is inside the area SHALL store its payload bytes there, starting at that offset. Bytes that would fall past `0x66FF` SHALL be dropped. The write SHALL NOT be forwarded to the flight loop, SHALL NOT produce non-active/non-writable warnings, and SHALL NOT count as a rejected write. Each write SHALL be logged at debug level with its offset, size and bytes.

#### Scenario: Write is stored
- **WHEN** a client writes the 8 bytes `01 02 03 04 05 06 07 08` to `0x66F8`
- **THEN** user-area bytes `0x66F8`–`0x66FF` hold those values, and nothing is forwarded or warned about

#### Scenario: Write past the end is clipped
- **WHEN** a client writes 8 bytes to `0x66FC`
- **THEN** the first 4 bytes are stored at `0x66FC`–`0x66FF` and the other 4 are dropped

### Requirement: User area answers reads

A read record whose `dwOffset` is inside the user area SHALL be answered from the stored bytes, starting at that offset. Any part of the payload past `0x66FF` SHALL be zero-filled. Such reads SHALL NOT consult the value table and SHALL NOT produce "not in table" warnings.

#### Scenario: Read back what was written
- **WHEN** a client writes `2A 00` to `0x66C0`, and later the same or another client reads 2 bytes from `0x66C0`
- **THEN** the payload is `2A 00`

#### Scenario: Unwritten bytes read as zero
- **WHEN** a client reads 8 bytes from `0x66F8` and nothing has been written there since the plugin started
- **THEN** the payload is eight zero bytes and no warning is logged

#### Scenario: Read past the end is zero-filled
- **WHEN** a client reads 8 bytes from `0x66FC` after `0x66FC`–`0x66FF` were set to `AA BB CC DD`
- **THEN** the payload is `AA BB CC DD 00 00 00 00`

### Requirement: User area lifetime

The user area SHALL be all zeros when the plugin starts. Its contents SHALL survive "Reload Mappings" and "Reset Warnings".

#### Scenario: Reload keeps the contents
- **WHEN** a client writes to `0x66C0` and the user then reloads mappings
- **THEN** a later read of `0x66C0` returns the written bytes

### Requirement: User area takes precedence over mappings

A `mappings.toml` entry whose offset is inside `0x66C0`–`0x66FF` SHALL be dropped when mappings are loaded, with a warning naming the offset.

#### Scenario: Mapping in the user area
- **WHEN** `mappings.toml` contains a mapping for offset `0x66D0`
- **THEN** the mapping is not resolved, a warning is logged, and `0x66D0` is served from the user area
