# FSUIPC IPC Protocol Specification

## Purpose

How `ipc_host` parses and answers the FSUIPC shared-memory request stream, following the FSUIPC SDK client libraries: record framing by `dwId`, reads, writes, the `0x8001` key write and the message result.

## Requirements

### Requirement: Record framing by dwId

The parser SHALL read records back to back from the start of the mapped view, choosing the header layout from the first DWORD (`dwId`, little-endian):

- `1` (Read32): 16-byte header `[dwId][dwOffset][nBytes][pDest:4]`, payload at +16
- `4` (Read64): 20-byte packed header `[dwId][dwOffset][nBytes][pDest:8]`, payload at +20
- `2` (Write): 12-byte header `[dwId][dwOffset][nBytes]`, payload at +12
- `0`: end of stream

The next record SHALL start right after the current record's `nBytes` payload bytes. `nBytes` SHALL be the full 32-bit field, with no flag bits.

#### Scenario: 32-bit reads
- **WHEN** the view contains `fixtures/slc-3-reads.bin`
- **THEN** the parser yields three Read32 records for `0x3304` (4 bytes), `0x3308` (4 bytes) and `0x3124` (1 byte), with no errors

#### Scenario: Write followed by terminator
- **WHEN** the view contains `fixtures/fsinterrogate-offset-8001.bin`
- **THEN** the parser yields exactly one Write record for `0x8001` with 13 bytes whose payload starts at view offset `0x0C`, then stops at the terminator at `0x19` with no errors

#### Scenario: 64-bit read
- **WHEN** the view contains a Read64 record for `0x0238` with 2 bytes, followed by a zero `dwId`
- **THEN** the parser yields one Read64 record whose payload starts at view offset 20, with no errors

#### Scenario: Mixed stream
- **WHEN** the view contains Read32 `0x3304`/4, Write `0x0BC0`/2 and Read32 `0x3308`/4, then a terminator
- **THEN** the parser yields those three records in order, with no errors

### Requirement: Malformed views stop parsing

The parser SHALL stop and count one error when it finds an unknown `dwId`, when a header or payload would extend past the end of the view, or when the view ends without a terminator. It SHALL NOT scan forward looking for later records.

#### Scenario: Unknown dwId
- **WHEN** a record has `dwId = 7`
- **THEN** parsing stops at that record, one error is counted, and earlier records have already been processed

#### Scenario: Payload overruns view
- **WHEN** a Read32 record declares more `nBytes` than remain in the view
- **THEN** parsing stops, one error is counted and nothing is written beyond the view

### Requirement: Answering reads

For each read record whose offset is in the value table, the server SHALL write the value into the record's payload, never writing more than `nBytes` bytes. Payload bytes the server can't fill SHALL be zero: all `nBytes` for a read whose offset isn't in the table or whose `dwOffset` is above `0xFFFF`, and the bytes past the value for a read wider than the offset's value.

#### Scenario: Unknown offset zero-filled
- **WHEN** a client reads 2 bytes from an offset not in the table and its payload holds `0xAB 0xCD`
- **THEN** the payload holds `0x00 0x00` after processing

### Requirement: Applying writes

A write record whose offset is active and writable in the value table SHALL be decoded by the entry's type and forwarded to the flight loop. Writes to inactive or non-writable offsets SHALL be ignored with the existing once-per-offset warnings. A write with `dwOffset` above `0xFFFF` SHALL be ignored and SHALL NOT count as an error.

#### Scenario: Write to writable offset
- **WHEN** a Write record targets a writable `u16` offset with 2 bytes
- **THEN** a write request with the decoded value is sent to the flight loop

### Requirement: Application key write at 0x8001

A write to offset `0x8001` SHALL be treated as an application key registration. It SHALL NOT be forwarded, SHALL NOT produce non-active/non-writable warnings, and SHALL NOT count as an error. The server SHALL log the key text (printable ASCII up to the first NUL, limited to `nBytes`) at the level set by `key_write` in `config.toml`'s `[log_levels]` table. Accepted values are `off`, `error`, `warn`, `info`, `debug` and `trace`. A missing setting SHALL mean `info`. An invalid value SHALL log a warning and fall back to `info`. The setting SHALL be re-applied when the config is reloaded.

The older `[settings] key_write_log_level` SHALL still be read as an alias. If both are set, `[log_levels] key_write` SHALL win and a deprecation warning SHALL be logged.

#### Scenario: Key write logged
- **WHEN** a client writes the 13 bytes `"6PETEXPDRVW3\0"` to `0x8001` and `[log_levels] key_write` is `info`
- **THEN** an info-level log line containing `6PETEXPDRVW3` is emitted, and no write is forwarded

#### Scenario: Key write silenced
- **WHEN** `[log_levels] key_write = "off"`
- **THEN** the same write produces no log line

#### Scenario: Old setting still honoured
- **WHEN** `config.toml` has `[settings] key_write_log_level = "off"` and no `[log_levels] key_write`
- **THEN** key writes produce no log line

#### Scenario: New setting wins
- **WHEN** `config.toml` has both `[settings] key_write_log_level = "off"` and `[log_levels] key_write = "info"`
- **THEN** key writes are logged at info, and a deprecation warning about `key_write_log_level` is logged when the config loads

### Requirement: Message result

The window procedure SHALL return `FS6IPC_MESSAGE_SUCCESS` (1) after processing a view that parsed without errors, and `FS6IPC_MESSAGE_FAILURE` (0) after processing a view that was malformed. Rejected writes alone SHALL NOT make the result a failure.

#### Scenario: Clean view
- **WHEN** a view parses with zero errors
- **THEN** the message result is 1

#### Scenario: Malformed view
- **WHEN** a view contains an unknown `dwId`
- **THEN** the message result is 0
