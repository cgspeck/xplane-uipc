## ADDED Requirements

### Requirement: Extended sentinel recovery scan

**WHEN** a bad sentinel is encountered in `iterate_records` and the 16-byte
`find_next_record` scan (Phase 1) fails to find a valid "luaP" sentinel,
**THEN** a second scan SHALL be performed covering the remaining bytes up to
`view_size`, bounded by `view_size - sentinel_offset - 1 - 12`.

**IF** the second scan (Phase 2) finds a valid sentinel, **THEN** the parser
SHALL advance to that position and continue processing.

**IF** Phase 2 also fails, **THEN** the parser SHALL break (same as current
Phase-1-fail behavior — true end of data).

#### Scenario: Recovery through large junk region

- **GIVEN** a mapped view containing: orphan record, `:FSD` string, 100 bytes of
  zeros, then a valid record with "luaP" sentinel
- **WHEN** the parser encounters the bad sentinel and Phase 1 fails
- **THEN** Phase 2 SHALL find the valid "luaP" sentinel
- **AND** the valid record SHALL be processed normally

#### Scenario: No recovery when no valid sentinel exists

- **GIVEN** a mapped view ending with garbage and no "luaP" sentinel
- **WHEN** the parser encounters a bad sentinel and Phase 2 fails
- **THEN** the parser SHALL break and return

### Requirement: Once-logging of bad sentinel hex values

The system SHALL log the hex value of each distinct bad sentinel at most once
per process lifetime.

**WHEN** a bad sentinel is detected, **THEN** the system SHALL check an
`AtomicBool` guard for that sentinel value. **IF** not yet logged, **THEN** the
system SHALL log `"Bad sentinel at offset {:#x}: value {:#010x}"` at `warn`
level and set the guard.

The guard uses an array or map keyed by the sentinel u32 value, or — for
simplicity — two separate `AtomicBool` values: one for the `:FSD` sentinel
(`0x4453463A`) and one for any other unrecognised value. If a third distinct
sentinel value appears after the "other" guard is set, it goes un-logged.

#### Scenario: First bad sentinel logged

- **GIVEN** two records with the same unknown sentinel value `0xBAD5EED`
- **WHEN** the parser processes the view
- **THEN** `"Bad sentinel … value 0x00BAD5EED"` SHALL be logged exactly once

### Requirement: `:FSD` trailing text logging

**WHEN** a bad sentinel's 4 bytes match `0x4453463A` (`:FSD ` in little-endian),
**THEN** the system SHALL extract trailing printable ASCII text starting from
the byte after the sentinel, up to 255 bytes, stopping at the first null byte
or non-printable character (char < 0x20 or > 0x7E).

**IF** at least one printable character was extracted, **THEN** the system SHALL
log once at `info` level: `"Bad sentinel at offset {:#x}: ':FSD' followed by:
\"{text}\""`

**IF** no printable characters follow (immediate null or non-printable),
**THEN** no text log SHALL be produced.

The `:FSD` text log SHALL use its own `AtomicBool` guard, independent of the
hex-value guard.

#### Scenario: `:FSD` text logged on first occurrence

- **GIVEN** two views, each containing `:FSDX_GSBOARDING_STATE` before zeros
- **WHEN** the parser processes both views
- **THEN** `"':FSD' followed by: \"X_GSBOARDING_STATE\""` SHALL be logged on the
  first occurrence only

#### Scenario: `:FSD` without trailing text

- **GIVEN** a view with `:FSD` immediately followed by null bytes
- **WHEN** the parser encounters the bad sentinel
- **THEN** no `:FSD`-text log SHALL be produced

### Requirement: `view_size` parameter for `iterate_records`

`iterate_records` SHALL accept a `view_size: usize` parameter providing the
total byte count of the mapped view.

- `process_mapped_view` SHALL accept and forward `view_size`
- `wnd_proc` SHALL pass the `VirtualQuery` `RegionSize` as `view_size`
- `capture-inspect` SHALL pass `data.len()` as `view_size`

**WHEN** `view_size` is smaller than the current cursor position + 16,
**THEN** Phase 2 scanning SHALL safely clamp to zero (via
`saturating_sub`), producing the same result as Phase 1 failure — the loop
breaks.

### Requirement: Existing behavior preserved

All existing tests SHALL pass without modification. The new `view_size`
parameter SHALL be the only signature change.
